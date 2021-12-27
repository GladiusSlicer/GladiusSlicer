use crate::plotter::monotone::get_monotone_sections;
use crate::settings::LayerSettings;
use crate::types::{Move, MoveChain, MoveType};

use geo::prelude::*;
use geo::*;
use geo_clipper::*;

pub fn partial_fill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_ratio: f64,
) -> Vec<MoveChain> {
    spaced_fill_polygon(
        poly,
        settings,
        MoveType::Infill,
        settings.layer_width / fill_ratio,
    )
}

pub fn solid_fill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_type: MoveType,
) -> Vec<MoveChain> {
    spaced_fill_polygon(poly, settings, fill_type, settings.layer_width)
}

pub fn spaced_fill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_type: MoveType,
    spacing: f64,
) -> Vec<MoveChain> {
    poly.offset(
        ((-settings.layer_width / 2.0) * (1.0 - settings.infill_perimeter_overlap_percentage))
            + (settings.layer_width / 2.0),
        JoinType::Square,
        EndType::ClosedPolygon,
        100000.0,
    )
    .iter()
    .filter(|poly| poly.unsigned_area() > 1.0)
    .map(|poly| {
        get_monotone_sections(poly)
            .iter()
            .filter_map(|section| {
                let mut current_y = ((section.left_chain[0].y) / spacing).floor() * spacing;

                let mut orient = true;

                let mut start_point = None;

                let mut line_change = true;

                let mut left_index = 0;
                let mut right_index = 0;

                let mut moves = vec![];

                loop {
                    while left_index < section.left_chain.len()
                        && section.left_chain[left_index].y > current_y
                    {
                        left_index += 1;
                        line_change = true;
                    }

                    if left_index == section.left_chain.len() {
                        break;
                    }

                    while right_index < section.right_chain.len()
                        && section.right_chain[right_index].y > current_y
                    {
                        right_index += 1;
                        line_change = true;
                    }

                    if right_index == section.right_chain.len() {
                        break;
                    }

                    let left_top = section.left_chain[left_index - 1];
                    let left_bot = section.left_chain[left_index];
                    let right_top = section.right_chain[right_index - 1];
                    let right_bot = section.right_chain[right_index];

                    let left_point = point_lerp(&left_top, &left_bot, current_y);
                    let right_point = point_lerp(&right_top, &right_bot, current_y);

                    start_point = start_point.or(Some(Coordinate {
                        x: left_point.x,
                        y: current_y,
                    }));

                    if orient {
                        moves.push(Move {
                            end: Coordinate {
                                x: left_point.x,
                                y: current_y,
                            },
                            move_type: if line_change {
                                MoveType::Travel
                            } else {
                                fill_type
                            },
                            width: settings.layer_width,
                        });

                        moves.push(Move {
                            end: Coordinate {
                                x: right_point.x,
                                y: current_y,
                            },
                            move_type: fill_type,
                            width: settings.layer_width,
                        });
                    } else {
                        moves.push(Move {
                            end: Coordinate {
                                x: right_point.x,
                                y: current_y,
                            },
                            move_type: if line_change {
                                MoveType::Travel
                            } else {
                                fill_type
                            },
                            width: settings.layer_width,
                        });

                        moves.push(Move {
                            end: Coordinate {
                                x: left_point.x,
                                y: current_y,
                            },
                            move_type: fill_type,
                            width: settings.layer_width,
                        });
                    }

                    orient = !orient;
                    current_y -= spacing;
                    line_change = false;
                }

                start_point.map(|start_point| MoveChain { start_point, moves })
            })
            .collect::<Vec<_>>()
            .into_iter()
    })
    .flatten()
    .collect()
}

#[inline]
fn point_lerp(a: &Coordinate<f64>, b: &Coordinate<f64>, y: f64) -> Coordinate<f64> {
    Coordinate {
        x: lerp(a.x, b.x, (y - a.y) / (b.y - a.y)),
        y,
    }
}

#[inline]
fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + f * (b - a)
}
