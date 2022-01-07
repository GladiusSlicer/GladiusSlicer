mod infill;
mod monotone;
mod perimeter;
pub mod polygon_operations;
mod support;

pub use crate::plotter::infill::*;
use crate::plotter::perimeter::*;
use crate::plotter::polygon_operations::PolygonOperations;
use crate::settings::{LayerSettings, SkirtSettings};
use crate::types::{Command, Move, MoveChain, MoveType};
use crate::{Object, Settings, StateChange};
use geo::coordinate_position::CoordPos;
use geo::coordinate_position::CoordinatePosition;
use geo::prelude::*;
use geo::simplifyvw::SimplifyVWPreserve;
use geo::*;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use std::iter::FromIterator;

pub struct Slice {
    main_polygon: MultiPolygon<f64>,
    remaining_area: MultiPolygon<f64>,
    support_interface: Option<MultiPolygon<f64>>,
    support_tower: Option<MultiPolygon<f64>>,
    fixed_chains: Vec<MoveChain>,
    chains: Vec<MoveChain>,
    pub bottom_height: f64,
    pub top_height: f64,
    layer_settings: LayerSettings,
}

impl Slice {
    pub fn from_single_point_loop<I>(
        line: I,
        bottom_height: f64,
        top_height: f64,
        layer_count: usize,
        settings: &Settings,
    ) -> Self
    where
        I: Iterator<Item = (f64, f64)>,
    {
        let polygon = Polygon::new(LineString::from_iter(line), vec![]);

        let layer_settings =
            settings.get_layer_settings(layer_count, (bottom_height + top_height) / 2.0);

        Slice {
            main_polygon: MultiPolygon(vec![polygon.simplifyvw_preserve(&0.01)]),
            remaining_area: MultiPolygon(vec![polygon]),
            support_interface: None,
            support_tower: None,
            fixed_chains: vec![],
            chains: vec![],
            bottom_height,
            top_height,
            layer_settings,
        }
    }

    pub fn from_multiple_point_loop(
        lines: MultiLineString<f64>,
        bottom_height: f64,
        top_height: f64,
        layer_count: usize,
        settings: &Settings,
    ) -> Self {
        let mut lines_and_area: Vec<(LineString<f64>, f64)> = lines
            .into_iter()
            .map(|line| {
                let area = line
                    .clone()
                    .into_points()
                    .iter()
                    .circular_tuple_windows::<(_, _)>()
                    .map(|(p1, p2)| (p1.x() + p2.x()) * (p2.y() - p1.y()))
                    .sum();
                (line, area)
            })
            .collect();

        lines_and_area.sort_by(|(_l1, a1), (_l2, a2)| a2.partial_cmp(a1).unwrap());
        let mut polygons = vec![];

        for (line, area) in lines_and_area {
            if area > 0.0 {
                polygons.push(Polygon::new(line.clone(), vec![]));
            } else {
                //counter clockwise interior polygon
                let smallest_polygon = polygons
                    .iter_mut()
                    .rev()
                    .find(|poly| poly.contains(&line.0[0]))
                    .expect("Polygon order failure");
                smallest_polygon.interiors_push(line);
            }
        }

        let multi_polygon: MultiPolygon<f64> = MultiPolygon(polygons);

        let layer_settings =
            settings.get_layer_settings(layer_count, (bottom_height + top_height) / 2.0);

        Slice {
            main_polygon: multi_polygon.simplifyvw_preserve(&0.01),
            remaining_area: multi_polygon,
            support_interface: None,
            support_tower: None,
            chains: vec![],
            fixed_chains: vec![],
            bottom_height,
            top_height,
            layer_settings,
        }
    }

    pub fn get_height(&self) -> f64 {
        (self.bottom_height + self.top_height) / 2.0
    }

    pub fn get_entire_slice_polygon(&self) -> &MultiPolygon<f64> {
        &self.main_polygon
    }
    pub fn get_support_polygon(&self) -> MultiPolygon<f64> {
        match (self.support_tower.clone(), self.support_interface.clone()) {
            (None, None) => MultiPolygon(vec![]),
            (Some(tower), None) => tower,
            (None, Some(interface)) => interface,
            (Some(tower), Some(interface)) => tower.union_with(&interface),
        }
    }

    pub fn slice_perimeters_into_chains(&mut self, number_of_perimeters: usize) {
        if let Some(mc) = inset_polygon_recursive(
            &self.remaining_area,
            &self.layer_settings,
            true,
            number_of_perimeters - 1,
        ) {
            self.fixed_chains.push(mc);
        }

        self.remaining_area = self
            .remaining_area
            .offset_from(-self.layer_settings.layer_width * number_of_perimeters as f64);
    }

    pub fn shrink_layer(&mut self) {
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

    pub fn fill_remaining_area(&mut self, solid: bool, layer_count: usize) {
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
                    &poly,
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
    }

    pub fn fill_solid_subtracted_area(&mut self, other: &MultiPolygon<f64>, layer_count: usize) {
        //For each area not in this slice that is in the other polygon, fill solid

        let solid_area = self
            .remaining_area
            .difference_with(other)
            .offset_from(self.layer_settings.layer_width * 4.0)
            .intersection_with(&self.remaining_area);

        let angle = 45.0 + (120_f64) * layer_count as f64;

        let layer_settings = &self.layer_settings;
        self.chains.extend(
            &mut solid_area
                .0
                .iter()
                .map(|poly| {
                    linear_fill_polygon(&poly, layer_settings, MoveType::SolidInfill, angle)
                        .into_iter()
                })
                .flatten(),
        );

        self.remaining_area = self.remaining_area.difference_with(&solid_area)
    }

    pub fn fill_solid_bridge_area(&mut self, layer_below: &MultiPolygon<f64>) {
        //For each area not in this slice that is in the other polygon, fill solid

        let solid_area = self
            .remaining_area
            .difference_with(layer_below)
            .offset_from(self.layer_settings.layer_width * 4.0)
            .intersection_with(&self.remaining_area);

        let layer_settings = &self.layer_settings;
        self.chains.extend(
            &mut solid_area
                .0
                .iter()
                .map(|poly| {
                    let unsupported_area: MultiPolygon<f64> = poly.difference_with(layer_below);
                    let mut angle = get_optimal_bridge_angle(poly, &unsupported_area);

                    if angle < 0.0 {
                        angle += 180.0;
                    }

                    linear_fill_polygon(poly, layer_settings, MoveType::Bridging, angle).into_iter()
                })
                .flatten(),
        );

        self.remaining_area = self.remaining_area.difference_with(&solid_area)
    }

    pub fn fill_solid_top_layer(&mut self, layer_above: &MultiPolygon<f64>, layer_count: usize) {
        //For each area not in this slice that is in the other polygon, fill solid

        let solid_area = self
            .remaining_area
            .difference_with(layer_above)
            .offset_from(self.layer_settings.layer_width * 4.0)
            .intersection_with(&self.remaining_area);

        for poly in &solid_area {
            let angle = 45.0 + (120_f64) * layer_count as f64;

            let new_moves =
                linear_fill_polygon(&poly, &self.layer_settings, MoveType::TopSolidInfill, angle);

            for chain in new_moves {
                self.chains.push(chain);
            }
        }

        self.remaining_area = self.remaining_area.difference_with(&solid_area)
    }

    pub fn generate_skirt(
        &mut self,
        convex_polygon: &Polygon<f64>,
        skirt_settings: &SkirtSettings,
    ) {
        let offset_hull_multi = convex_polygon.offset_from(skirt_settings.distance);

        assert_eq!(offset_hull_multi.0.len(), 1);

        let moves = offset_hull_multi.0[0]
            .exterior()
            .0
            .iter()
            .circular_tuple_windows::<(_, _)>()
            .map(|(&_start, &end)| Move {
                end,
                move_type: MoveType::OuterPerimeter,
                width: self.layer_settings.layer_width,
            })
            .collect();

        self.fixed_chains.push(MoveChain {
            start_point: offset_hull_multi.0[0].exterior()[0],
            moves,
        });
    }
    pub fn generate_brim(&mut self, entire_first_layer: MultiPolygon<f64>, brim_width: f64) {
        let layer_settings = &self.layer_settings;
        self.fixed_chains.extend(
            (0..((brim_width / self.layer_settings.layer_width).floor() as usize))
                .rev()
                .map(|i| {
                    (i as f64 * layer_settings.layer_width) + (layer_settings.layer_width / 2.0)
                })
                .map(|distance| entire_first_layer.offset_from(distance))
                .map(|multi| {
                    multi.into_iter().map(|poly| {
                        let moves = poly
                            .exterior()
                            .0
                            .iter()
                            .circular_tuple_windows::<(_, _)>()
                            .map(|(&_start, &end)| Move {
                                end,
                                move_type: MoveType::OuterPerimeter,
                                width: layer_settings.layer_width,
                            })
                            .collect();

                        MoveChain {
                            start_point: poly.exterior()[0],
                            moves,
                        }
                    })
                })
                .flatten(),
        );
    }

    pub fn order_chains(&mut self) {
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
                                .unwrap()
                                .moves
                                .last()
                                .unwrap()
                                .end
                                .euclidean_distance(&a.start_point),
                        )
                    })
                    .unwrap();
                let closest_chain = self.chains.remove(index);
                ordered_chains.push(closest_chain);
            }

            ordered_chains
        } else {
            vec![]
        };

        self.chains = ordered_chains;
    }

    pub fn slice_into_commands(&mut self, commands: &mut Vec<Command>, layer_thickness: f64) {
        if !self.fixed_chains.is_empty() {
            let mut full_moves = vec![];
            let starting_point = self.fixed_chains[0].start_point;
            for chain in self.fixed_chains.iter_mut().chain(self.chains.iter_mut()) {
                full_moves.push(Move {
                    end: chain.start_point,
                    move_type: MoveType::Travel,
                    width: 0.0,
                });
                full_moves.append(&mut chain.moves)
            }

            commands.append(
                &mut MoveChain {
                    moves: full_moves,
                    start_point: starting_point,
                }
                .create_commands(&self.layer_settings, layer_thickness),
            );
        }
    }
}

fn get_optimal_bridge_angle(fill_area: &Polygon<f64>, unsupported_area: &MultiPolygon<f64>) -> f64 {
    let unsuported_lines: Vec<_> = unsupported_area
        .iter()
        .map(|poly| std::iter::once(poly.exterior()).chain(poly.interiors().iter()))
        .flatten()
        .map(|line_string| {
            line_string
                .0
                .iter()
                .circular_tuple_windows::<(&Coordinate<f64>, &Coordinate<f64>)>()
        })
        .flatten()
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
        .min_by(|(_, l_sum), (_, r_sum)| l_sum.partial_cmp(r_sum).unwrap())
        .map(|((x, y), _)| -90.0 - (y).atan2(x).to_degrees())
        .unwrap_or(0.0)
}

pub fn convert_objects_into_moves(objects: Vec<Object>, settings: &Settings) -> Vec<Command> {
    println!("Convert into Commnds");
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
                    let layer_settings = settings.get_layer_settings(layer_num, slice.top_height);
                    let mut moves = vec![];
                    moves.push(Command::ChangeObject { object: object_num });
                    moves.push(Command::LayerChange {
                        z: slice.top_height,
                    });
                    moves.push(Command::SetState {
                        new_state: StateChange {
                            extruder_temp: Some(layer_settings.extruder_temp),
                            bed_temp: Some(layer_settings.bed_temp),
                            fan_speed: Some(if layer_num < settings.fan.disable_fan_for_layers {
                                0.0
                            } else {
                                settings.fan.fan_speed
                            }),
                            movement_speed: None,
                            acceleration: None,
                            retract: None,
                        },
                    });
                    slice.slice_into_commands(&mut moves, slice.top_height - last_layer);

                    last_layer = slice.top_height;
                    (slice.top_height, moves)
                })
                .collect::<Vec<(f64, Vec<Command>)>>()
        })
        .map(|a| a.into_iter())
        .flatten()
        .collect();

    layer_moves
        .sort_by(|(a, _), (b, _)| a.partial_cmp(b).expect("No NAN layer heights are allowed"));

    layer_moves
        .into_iter()
        .map(|(_, layer_moves)| layer_moves)
        .flatten()
        .collect()
}
