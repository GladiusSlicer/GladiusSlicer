mod infill;
pub(crate) mod lightning_infill;
mod monotone;
mod perimeter;
pub mod polygon_operations;
pub(crate) mod support;

pub use crate::plotter::infill::*;
use crate::plotter::perimeter::inset_polygon_recursive;
use crate::plotter::polygon_operations::PolygonOperations;
use crate::utils::point_lerp;
use crate::{Object, Settings, StateChange};
use coordinate_position::CoordPos;
use geo::prelude::*;
use geo::{Coord, MultiPolygon, Polygon};
use gladius_shared::settings::SkirtSettings;
use gladius_shared::types::{Command, Move, MoveChain, MoveType, RetractionType, Slice};
use itertools::Itertools;
use log::info;
use ordered_float::OrderedFloat;

pub trait Plotter {
    fn slice_perimeters_into_chains(&mut self, number_of_perimeters: usize);
    fn shrink_layer(&mut self);
    fn fill_remaining_area(&mut self, solid: bool, layer_count: usize);
    fn fill_solid_subtracted_area(&mut self, other: &MultiPolygon<f64>, layer_count: usize);
    fn fill_solid_bridge_area(&mut self, layer_below: &MultiPolygon<f64>);
    fn fill_solid_top_layer(&mut self, layer_above: &MultiPolygon<f64>, layer_count: usize);
    fn generate_skirt(
        &mut self,
        convex_polygon: &Polygon<f64>,
        skirt_settings: &SkirtSettings,
        settings: &Settings,
    );
    fn generate_brim(&mut self, entire_first_layer: MultiPolygon<f64>, brim_width: f64);
    fn order_chains(&mut self);
    fn slice_into_commands(&mut self, commands: &mut Vec<Command>, layer_thickness: f64);
}

impl Plotter for Slice {
    fn slice_perimeters_into_chains(&mut self, number_of_perimeters: usize) {
        let mut new_chains = self
            .remaining_area
            .iter()
            .map(|poly| MultiPolygon(vec![poly.clone()]))
            .filter_map(|multi| {
                inset_polygon_recursive(
                    &multi,
                    &self.layer_settings,
                    true,
                    number_of_perimeters - 1,
                )
            })
            .collect::<Vec<_>>();

        self.fixed_chains.append(&mut new_chains);

        let perimeter_inset = if number_of_perimeters == 0 {
            0.0
        } else if number_of_perimeters == 1 {
            self.layer_settings
                .extrusion_width
                .exterior_surface_perimeter
        } else {
            self.layer_settings
                .extrusion_width
                .exterior_surface_perimeter
                + ((number_of_perimeters - 1) as f64
                    * self.layer_settings.extrusion_width.exterior_inner_perimeter)
        };

        self.remaining_area = self.remaining_area.offset_from(-perimeter_inset);
    }

    fn shrink_layer(&mut self) {
        if let Some(shrink_ammount) = self.layer_settings.layer_shrink_amount {
            self.support_tower = self
                .support_tower
                .as_ref()
                .map(|tower| tower.offset_from(-shrink_ammount));
            self.support_interface = self
                .support_interface
                .as_ref()
                .map(|interface| interface.offset_from(-shrink_ammount));
            self.remaining_area = self.remaining_area.offset_from(-shrink_ammount);
        }
    }

    fn fill_remaining_area(&mut self, solid: bool, layer_count: usize) {
        //For each region still available fill wih infill
        for poly in &self.remaining_area {
            if solid {
                let new_moves = solid_infill_polygon(
                    poly,
                    &self.layer_settings,
                    MoveType::SolidInfill,
                    layer_count,
                    self.get_height(),
                );

                for chain in new_moves {
                    self.chains.push(chain);
                }
            } else {
                let new_moves = partial_infill_polygon(
                    poly,
                    &self.layer_settings,
                    self.layer_settings.infill_percentage,
                    layer_count,
                    self.get_height(),
                );

                for chain in new_moves {
                    self.chains.push(chain);
                }
            }
        }

        self.remaining_area = MultiPolygon(vec![]);
    }

    fn fill_solid_subtracted_area(&mut self, other: &MultiPolygon<f64>, layer_count: usize) {
        //For each area not in this slice that is in the other polygon, fill solid

        let solid_area = self
            .remaining_area
            .difference_with(other)
            .offset_from(self.layer_settings.extrusion_width.solid_infill * 4.0)
            .intersection_with(&self.remaining_area);

        let angle = 45.0 + (120_f64) * layer_count as f64;

        let layer_settings = &self.layer_settings;
        self.chains
            .extend(&mut solid_area.0.iter().flat_map(|poly| {
                linear_fill_polygon(poly, layer_settings, MoveType::SolidInfill, angle).into_iter()
            }));

        self.remaining_area = self.remaining_area.difference_with(&solid_area);
    }

    fn fill_solid_bridge_area(&mut self, layer_below: &MultiPolygon<f64>) {
        //For each area not in this slice that is in the other polygon, fill solid

        let solid_area = self
            .remaining_area
            .difference_with(layer_below)
            .offset_from(self.layer_settings.extrusion_width.bridge * 4.0)
            .intersection_with(&self.remaining_area);

        let layer_settings = &self.layer_settings;
        self.chains
            .extend(&mut solid_area.0.iter().flat_map(|poly| {
                let unsupported_area: MultiPolygon<f64> = poly.difference_with(layer_below);
                let mut angle = get_optimal_bridge_angle(poly, &unsupported_area);

                if angle < 0.0 {
                    angle += 180.0;
                }

                linear_fill_polygon(poly, layer_settings, MoveType::Bridging, angle).into_iter()
            }));

        self.remaining_area = self.remaining_area.difference_with(&solid_area);
    }

    fn fill_solid_top_layer(&mut self, layer_above: &MultiPolygon<f64>, layer_count: usize) {
        //For each area not in this slice that is in the other polygon, fill solid

        let solid_area = self
            .remaining_area
            .difference_with(layer_above)
            .offset_from(self.layer_settings.extrusion_width.solid_top_infill * 4.0)
            .intersection_with(&self.remaining_area);

        for poly in &solid_area {
            let angle = 45.0 + (120_f64) * layer_count as f64;

            let new_moves =
                linear_fill_polygon(poly, &self.layer_settings, MoveType::TopSolidInfill, angle);

            for chain in new_moves {
                self.chains.push(chain);
            }
        }

        self.remaining_area = self.remaining_area.difference_with(&solid_area);
    }

    fn generate_skirt(
        &mut self,
        convex_polygon: &Polygon<f64>,
        skirt_settings: &SkirtSettings,
        settings: &Settings,
    ) {
        let offset_hull_multi = convex_polygon.offset_from(skirt_settings.distance);

        assert_eq!(offset_hull_multi.0.len(), 1);

        let moves = offset_hull_multi.0[0]
            .exterior()
            .0
            .iter()
            .circular_tuple_windows::<(_, _)>()
            .map(|(&_start, &end)| {
                let bounded_endpoint = Coord {
                    x: end.x.max(0.0).min(settings.print_x),
                    y: end.y.max(0.0).min(settings.print_y),
                };

                Move {
                    end: bounded_endpoint,
                    move_type: MoveType::ExteriorSurfacePerimeter,
                    width: self
                        .layer_settings
                        .extrusion_width
                        .exterior_surface_perimeter,
                }
            })
            .collect();

        let start_point = Coord {
            x: offset_hull_multi.0[0].exterior()[0]
                .x
                .max(0.0)
                .min(settings.print_x),
            y: offset_hull_multi.0[0].exterior()[0]
                .y
                .max(0.0)
                .min(settings.print_y),
        };

        self.fixed_chains.push(MoveChain {
            start_point,
            moves,
            is_loop: true,
        });
    }

    fn generate_brim(&mut self, entire_first_layer: MultiPolygon<f64>, brim_width: f64) {
        let layer_settings = &self.layer_settings;
        self.fixed_chains.extend(
            (0..((brim_width
                / self
                    .layer_settings
                    .extrusion_width
                    .exterior_surface_perimeter)
                .floor() as usize))
                .rev()
                .map(|i| {
                    (i as f64 * layer_settings.extrusion_width.exterior_surface_perimeter)
                        + (layer_settings.extrusion_width.exterior_surface_perimeter / 2.0)
                })
                .map(|distance| entire_first_layer.offset_from(distance))
                .flat_map(|multi| {
                    multi.into_iter().map(|poly| {
                        let moves = poly
                            .exterior()
                            .0
                            .iter()
                            .circular_tuple_windows::<(_, _)>()
                            .map(|(&_start, &end)| Move {
                                end,
                                move_type: MoveType::ExteriorSurfacePerimeter,
                                width: layer_settings.extrusion_width.exterior_surface_perimeter,
                            })
                            .collect();

                        MoveChain {
                            start_point: poly.exterior()[0],
                            moves,
                            is_loop: true,
                        }
                    })
                }),
        );
    }

    fn order_chains(&mut self) {
        //Order Chains for fastest print
        let ordered_chains = if !self.chains.is_empty() {
            let mut ordered_chains = vec![self.chains.swap_remove(0)];

            while !self.chains.is_empty() {
                let index = self
                    .chains
                    .iter()
                    .position_min_by_key(|a| {
                        OrderedFloat(
                            ordered_chains
                                .last()
                                .expect("Chains is tests not to be empty")
                                .moves
                                .last()
                                .expect("chain should contain moves")
                                .end
                                .euclidean_distance(&a.start_point),
                        )
                    })
                    .expect("Chains is tests not to be empty");
                let closest_chain = self.chains.remove(index);
                ordered_chains.push(closest_chain);
            }

            ordered_chains
        } else {
            Vec::new()
        };

        self.chains = ordered_chains;
    }

    fn slice_into_commands(&mut self, commands: &mut Vec<Command>, layer_thickness: f64) {
        if !self.fixed_chains.is_empty() {
            commands.push(Command::SetState {
                new_state: StateChange {
                    extruder_temp: None,
                    bed_temp: None,
                    fan_speed: None,
                    movement_speed: None,
                    acceleration: None,
                    retract: RetractionType::Retract,
                },
            });

            for chain in self.fixed_chains.drain(..).chain(self.chains.drain(..)) {
                let retraction_length = self.layer_settings.retraction_length;
                let retract_command =
                    if let Some(retraction_wipe) = self.layer_settings.retraction_wipe.as_ref() {
                        let ordered: Vec<Coord<f64>> = if chain.is_loop {
                            //fixme this is bad
                            chain
                                .moves
                                .iter()
                                .rev()
                                .take_while(|m| m.move_type != MoveType::Travel)
                                .map(|m| m.end)
                                .collect::<Vec<_>>()
                                .into_iter()
                                .rev()
                                .collect_vec()
                        } else {
                            chain.moves.iter().rev().map(|m| m.end).collect_vec()
                        };

                        let mut remaining_distance = retraction_wipe.distance;
                        let mut wipe_moves = ordered
                            .iter()
                            .tuple_windows::<(_, _)>()
                            .map(|(cur_point, next_point)| {
                                let len: f64 = cur_point.euclidean_distance(next_point);

                                (len, cur_point, next_point)
                            })
                            .filter_map(|(len, cur_point, next_point)| {
                                if remaining_distance - len > 0.0 {
                                    remaining_distance -= len;
                                    Some((len, *next_point))
                                } else if remaining_distance > 0.0 {
                                    let ret = (
                                        remaining_distance,
                                        point_lerp(cur_point, next_point, remaining_distance / len),
                                    );
                                    remaining_distance -= len;
                                    Some(ret)
                                } else {
                                    None
                                }
                            })
                            .map(|(len, next_point)| {
                                let retaction_distance =
                                    len / retraction_wipe.distance * retraction_length;

                                (retaction_distance, next_point)
                            })
                            .collect::<Vec<_>>();
                        /*
                        if chain.is_loop && chain.moves.len() > 3{
                            if let [m2,m1,..] = ordered[ordered.len()-3..ordered.len()]{
                                if let Some(m0) = ordered.first() {
                                    //let m1 = chain.start_point ;
                                    //inset the first move
                                    let bisector = directional_unit_bisector_left(&m0, &m1, &m2);

                                    let scaled_bisector = bisector.scale(self.layer_settings.extrusion_width.exterior_surface_perimeter);

                                    let inset_point = Coord::from((m1.x - scaled_bisector.x,m1.y - scaled_bisector.y));

                                    println!("{:?} {:?} {:?} ",m0,m1,m2);
                                    println!("{:?} {:?} {:?} ",bisector,scaled_bisector,inset_point);

                                    wipe_moves.insert(0,(0.0,inset_point))


                                }
                            }
                        }*/

                        if remaining_distance > 0.0 {
                            if let Some((distance, _)) = wipe_moves.last_mut() {
                                *distance += remaining_distance / retraction_wipe.distance
                                    * retraction_length;
                            }
                        }

                        Command::SetState {
                            new_state: StateChange {
                                extruder_temp: None,
                                bed_temp: None,
                                fan_speed: None,
                                movement_speed: Some(retraction_wipe.speed),
                                acceleration: Some(retraction_wipe.acceleration),
                                retract: RetractionType::MoveRetract(wipe_moves),
                            },
                        }
                    } else {
                        Command::SetState {
                            new_state: StateChange {
                                bed_temp: None,
                                extruder_temp: None,
                                fan_speed: None,
                                movement_speed: Some(self.layer_settings.speed.travel),
                                acceleration: Some(self.layer_settings.acceleration.travel),
                                retract: RetractionType::Retract,
                            },
                        }
                    };

                commands.push(Command::MoveTo {
                    end: chain.start_point,
                });
                commands.append(&mut chain.create_commands(&self.layer_settings, layer_thickness));

                commands.push(retract_command);
            }
        }
    }
}

fn get_optimal_bridge_angle(fill_area: &Polygon<f64>, unsupported_area: &MultiPolygon<f64>) -> f64 {
    let unsuported_lines: Vec<_> = unsupported_area
        .iter()
        .flat_map(|poly| std::iter::once(poly.exterior()).chain(poly.interiors().iter()))
        .flat_map(|line_string| {
            line_string
                .0
                .iter()
                .circular_tuple_windows::<(&Coord<f64>, &Coord<f64>)>()
        })
        .filter(|(&s, &f)| {
            //test the midpoint if it supported
            let mid_point = (s + f) / 2.0;
            let supported = fill_area.coordinate_position(&mid_point) == CoordPos::Inside;
            //if midpoint is in the fill area, then it is supported
            !supported
        })
        .collect();

    unsuported_lines
        .iter()
        .filter_map(|(line_start, line_end)| {
            let x_diff = line_end.x - line_start.x;
            let y_diff = line_end.y - line_start.y;

            let per_vec = (y_diff, -x_diff);
            let per_vec_len = (((x_diff) * (x_diff)) + ((y_diff) * (y_diff))).sqrt();

            if per_vec_len != 0.0 {
                Some(
                    unsuported_lines
                        .iter()
                        .map(|(inner_start, inner_end)| {
                            let x_diff = inner_end.x - inner_start.x;
                            let y_diff = inner_end.y - inner_start.y;

                            //println!("vec ({},{})", x_diff, y_diff);

                            let inner_vec = (x_diff, y_diff);

                            let dot = (inner_vec.0 * per_vec.0) + (inner_vec.1 * per_vec.1);

                            (dot / per_vec_len).abs()
                        })
                        .sum(),
                )
            } else {
                None
            }
            .map(|projection_sum: f64| (per_vec, projection_sum))
        })
        .min_by(|(_, l_sum), (_, r_sum)| {
            l_sum
                .partial_cmp(r_sum)
                .expect("Sum should not contain NAN")
        })
        .map_or(0.0, |((x, y), _)| -90.0 - (y).atan2(x).to_degrees())
}

pub fn convert_objects_into_moves(objects: Vec<Object>, settings: &Settings) -> Vec<Command> {
    info!("Convert into Commnds");
    let mut layer_moves: Vec<(f64, Vec<Command>)> = objects
        .into_iter()
        .enumerate()
        .map(|(object_num, object)| {
            let mut last_layer = 0.0;

            object
                .layers
                .into_iter()
                .enumerate()
                .map(|(layer_num, mut slice)| {
                    let layer_settings = settings.get_layer_settings(layer_num as u32, slice.top_height);
                    let mut moves = vec![];
                    moves.push(Command::ChangeObject { object: object_num });
                    moves.push(Command::LayerChange {
                        z: slice.top_height,
                        index: layer_num,
                    });
                    moves.push(Command::SetState {
                        new_state: StateChange {
                            extruder_temp: Some(layer_settings.extruder_temp),
                            bed_temp: Some(layer_settings.bed_temp),
                            fan_speed: Some(if (layer_num as u32) < settings.fan.disable_fan_for_layers {
                                0.0
                            } else {
                                settings.fan.fan_speed
                            }),
                            movement_speed: None,
                            acceleration: None,
                            retract: RetractionType::NoRetract,
                        },
                    });
                    slice.slice_into_commands(&mut moves, slice.top_height - last_layer);

                    last_layer = slice.top_height;
                    (slice.top_height, moves)
                })
                .collect::<Vec<(f64, Vec<Command>)>>()
        })
        .flat_map(|a| a.into_iter())
        .collect();

    layer_moves
        .sort_by(|(a, _), (b, _)| a.partial_cmp(b).expect("No NAN layer heights are allowed"));

    layer_moves
        .into_iter()
        .flat_map(|(_, layer_moves)| layer_moves)
        .collect()
}
