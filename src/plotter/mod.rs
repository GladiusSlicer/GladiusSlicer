mod infill;
mod monotone;
mod perimeter;

use crate::plotter::infill::*;
use crate::plotter::perimeter::*;
use crate::settings::{LayerSettings, SkirtSettings};
use crate::types::{Command, Move, MoveChain, MoveType};
use geo::coordinate_position::CoordPos;
use geo::coordinate_position::CoordinatePosition;
use geo::prelude::*;
use geo::*;
use geo_clipper::*;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use std::iter::FromIterator;

pub struct Slice {
    MainPolygon: MultiPolygon<f64>,
    remaining_area: MultiPolygon<f64>,
    solid_infill: Option<MultiPolygon<f64>>,
    normal_infill: Option<MultiPolygon<f64>>,
    fixed_chains: Vec<MoveChain>,
    chains: Vec<MoveChain>,
}

impl Slice {
    pub fn from_single_point_loop<I>(line: I) -> Self
    where
        I: Iterator<Item = (f64, f64)>,
    {
        let polygon = Polygon::new(LineString::from_iter(line), vec![]);

        Slice {
            MainPolygon: MultiPolygon(vec![polygon.clone()]),
            remaining_area: MultiPolygon(vec![polygon]),
            solid_infill: None,
            normal_infill: None,
            fixed_chains: vec![],
            chains: vec![],
        }
    }

    pub fn from_multiple_point_loop(lines: MultiLineString<f64>) -> Self {
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

        Slice {
            MainPolygon: multi_polygon.clone(),
            remaining_area: multi_polygon,
            solid_infill: None,
            normal_infill: None,
            chains: vec![],
            fixed_chains: vec![],
        }
    }

    pub fn get_entire_slice_polygon(&self) -> &MultiPolygon<f64> {
        &self.MainPolygon
    }

    pub fn slice_perimeters_into_chains(
        &mut self,
        settings: &LayerSettings,
        number_of_perimeters: usize,
    ) {
        if let Some(mc) = inset_polygon_recursive(
            &self.remaining_area,
            settings,
            true,
            number_of_perimeters - 1,
        ) {
            self.fixed_chains.push(mc);
        }

        self.remaining_area = self.remaining_area.offset(
            -settings.layer_width * number_of_perimeters as f64,
            JoinType::Square,
            EndType::ClosedPolygon,
            100000.0,
        );
    }

    pub fn fill_remaining_area(
        &mut self,
        settings: &LayerSettings,
        solid: bool,
        layer_count: usize,
    ) {
        //For each region still available fill wih infill
        for poly in &self.remaining_area {
            if solid {
                let angle = 45.0 + (120_f64) * layer_count as f64;
                let rotate_poly = poly.rotate_around_point(angle, Point(Coordinate::zero()));

                let new_moves = solid_fill_polygon(&rotate_poly, settings, MoveType::SolidInfill);

                for mut chain in new_moves {
                    chain.rotate(-angle.to_radians());
                    self.chains.push(chain);
                }
            } else {
                let new_moves = partial_fill_polygon(&poly, settings, settings.infill_percentage);

                for mut chain in new_moves {
                    self.chains.push(chain);
                }
            }
        }
    }

    pub fn fill_solid_subtracted_area(
        &mut self,
        other: &MultiPolygon<f64>,
        settings: &LayerSettings,
        layer_count: usize,
    ) {
        //For each area not in this slice that is in the other polygon, fill solid

        let solid_area = self
            .remaining_area
            .difference(other, 100000.0)
            .offset(
                settings.layer_width * 4.0,
                JoinType::Square,
                EndType::ClosedPolygon,
                100000.0,
            )
            .intersection(&self.remaining_area, 100000.0);

        let angle = 45.0 + (120_f64) * layer_count as f64;

        self.chains.extend(
            &mut solid_area
                .0
                .iter()
                .map(|poly| {
                    let rotate_poly = poly.rotate_around_point(angle, Point(Coordinate::zero()));

                    solid_fill_polygon(&rotate_poly, settings, MoveType::SolidInfill)
                        .into_iter()
                        .map(|mut chain| {
                            chain.rotate(-angle.to_radians());
                            chain
                        })
                })
                .flatten(),
        );

        self.remaining_area = self.remaining_area.difference(&solid_area, 100000.0)
    }

    pub fn fill_solid_bridge_area(
        &mut self,
        layer_below: &MultiPolygon<f64>,
        settings: &LayerSettings,
    ) {
        //For each area not in this slice that is in the other polygon, fill solid

        let solid_area = self
            .remaining_area
            .difference(layer_below, 100000.0)
            .offset(
                settings.layer_width * 4.0,
                JoinType::Square,
                EndType::ClosedPolygon,
                100000.0,
            )
            .intersection(&self.remaining_area, 100000.0);

        self.chains.extend(
            &mut solid_area
                .0
                .iter()
                .map(|poly| {
                    let unsupported_area: MultiPolygon<f64> =
                        poly.difference(layer_below, 100000.0);
                    let mut angle = get_optimal_bridge_angle(poly, &unsupported_area);

                    if angle < 0.0 {
                        angle += 180.0;
                    }
                    //println!("angle {}", angle);

                    let rotate_poly = poly.rotate_around_point(angle, Point(Coordinate::zero()));

                    solid_fill_polygon(&rotate_poly, settings, MoveType::Bridging)
                        .into_iter()
                        .map(move |mut chain| {
                            chain.rotate(-angle.to_radians());
                            chain
                        })
                })
                .flatten(),
        );

        self.remaining_area = self.remaining_area.difference(&solid_area, 100000.0)
    }

    pub fn fill_solid_top_layer(
        &mut self,
        layer_above: &MultiPolygon<f64>,
        settings: &LayerSettings,
        layer_count: usize,
    ) {
        //For each area not in this slice that is in the other polygon, fill solid

        let solid_area = self
            .remaining_area
            .difference(layer_above, 100000.0)
            .offset(
                settings.layer_width * 4.0,
                JoinType::Square,
                EndType::ClosedPolygon,
                100000.0,
            )
            .intersection(&self.remaining_area, 100000.0);

        for poly in &solid_area {
            let angle = 45.0 + (120_f64) * layer_count as f64;

            let rotate_poly = poly.rotate_around_point(angle, Point(Coordinate::zero()));

            let new_moves = solid_fill_polygon(&rotate_poly, settings, MoveType::TopSolidInfill);

            for mut chain in new_moves {
                chain.rotate(-angle.to_radians());
                self.chains.push(chain);
            }
        }

        self.remaining_area = self.remaining_area.difference(&solid_area, 100000.0)
    }

    pub fn generate_skirt(
        &mut self,
        convex_polygon: &Polygon<f64>,
        settings: &LayerSettings,
        skirt_settings: &SkirtSettings,
    ) {
        let offset_hull_multi = convex_polygon.offset(
            skirt_settings.distance,
            JoinType::Square,
            EndType::ClosedPolygon,
            100000.0,
        );

        assert_eq!(offset_hull_multi.0.len(), 1);

        let moves = offset_hull_multi.0[0]
            .exterior()
            .0
            .iter()
            .circular_tuple_windows::<(_, _)>()
            .map(|(&_start, &end)| Move {
                end,
                move_type: MoveType::OuterPerimeter,
                width: settings.layer_width,
            })
            .collect();

        self.fixed_chains.push(MoveChain {
            start_point: offset_hull_multi.0[0].exterior()[0],
            moves,
        });
    }

    pub fn slice_into_commands(
        &mut self,
        settings: &LayerSettings,
        commands: &mut Vec<Command>,
        layer_thickness: f64,
    ) {
        //Order Chains for fastest print
        let mut ordered_chains = if !self.chains.is_empty() {
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
        }
        else{
            vec![]
        };

        let mut full_moves = vec![];
        let starting_point = self.fixed_chains[0].start_point;
        for chain in self
            .fixed_chains
            .iter_mut()
            .chain(ordered_chains.iter_mut())
        {
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
            .create_commands(settings, layer_thickness),
        );
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
