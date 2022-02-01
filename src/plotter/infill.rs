use crate::plotter::monotone::get_monotone_sections;
use gladius_shared::settings::LayerSettings;
use gladius_shared::types::{Move, MoveChain, MoveType, PartialInfillTypes};

use crate::PolygonOperations;
use geo::prelude::*;
use geo::*;

pub trait SolidInfillFill {
    fn fill(&self, filepath: &str) -> Vec<MoveChain>;
}

pub trait PartialInfillFill {
    fn fill(&self, filepath: &str) -> Vec<MoveChain>;
}

pub fn linear_fill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_type: MoveType,
    angle: f64,
) -> Vec<MoveChain> {
    let rotate_poly = poly.rotate_around_point(angle, Point(Coordinate::zero()));

    let mut new_moves: Vec<MoveChain> = rotate_poly
        .offset_from(
            ((-settings.layer_width / 2.0) * (1.0 - settings.infill_perimeter_overlap_percentage))
                + (settings.layer_width / 2.0),
        )
        .iter()
        .flat_map(|polygon| {
            spaced_fill_polygon(polygon, settings, fill_type, settings.layer_width, 0.0)
        })
        .collect();

    for chain in new_moves.iter_mut() {
        chain.rotate(-angle.to_radians());
    }

    new_moves
}

pub fn partial_linear_fill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_type: MoveType,
    spacing: f64,
    angle: f64,
    offset: f64,
) -> Vec<MoveChain> {
    let rotate_poly = poly.rotate_around_point(angle, Point(Coordinate::zero()));

    let mut new_moves: Vec<MoveChain> = rotate_poly
        .offset_from(
            ((-settings.layer_width / 2.0) * (1.0 - settings.infill_perimeter_overlap_percentage))
                + (settings.layer_width / 2.0),
        )
        .iter()
        .flat_map(|polygon| spaced_fill_polygon(polygon, settings, fill_type, spacing, offset))
        .collect();

    for chain in new_moves.iter_mut() {
        chain.rotate(-angle.to_radians());
    }

    new_moves
}

pub fn support_linear_fill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_type: MoveType,
    spacing: f64,
    angle: f64,
    offset: f64,
) -> Vec<MoveChain> {
    let rotate_poly = poly.rotate_around_point(angle, Point(Coordinate::zero()));

    let mut new_moves: Vec<MoveChain> = rotate_poly
        .offset_from(-settings.layer_width / 2.0)
        .iter()
        .flat_map(|polygon| spaced_fill_polygon(polygon, settings, fill_type, spacing, offset))
        .collect();

    for chain in new_moves.iter_mut() {
        chain.rotate(-angle.to_radians());
    }

    new_moves
}

pub fn solid_infill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_type: MoveType,
    layer_count: usize,
    _layer_height: f64,
) -> Vec<MoveChain> {
    let angle = 45.0 + (120_f64) * layer_count as f64;

    linear_fill_polygon(poly, settings, fill_type, angle)
}

pub fn partial_infill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_ratio: f64,
    _layer_count: usize,
    layer_height: f64,
) -> Vec<MoveChain> {
    if fill_ratio < f64::EPSILON {
        return vec![];
    }
    match settings.partial_infill_type {
        PartialInfillTypes::Linear => partial_linear_fill_polygon(
            poly,
            settings,
            MoveType::Infill,
            settings.layer_width / fill_ratio,
            0.0,
            0.0,
        ),
        PartialInfillTypes::Rectilinear => {
            let mut fill = partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                2.0 * settings.layer_width / fill_ratio,
                45.0,
                0.0,
            );
            fill.append(&mut partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                2.0 * settings.layer_width / fill_ratio,
                135.0,
                0.0,
            ));
            fill
        }
        PartialInfillTypes::Triangle => {
            let mut fill = partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                3.0 * settings.layer_width / fill_ratio,
                45.0,
                0.0,
            );
            fill.append(&mut partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                3.0 * settings.layer_width / fill_ratio,
                45.0 + 60.0,
                0.0,
            ));
            fill.append(&mut partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                3.0 * settings.layer_width / fill_ratio,
                45.0 + 120.0,
                0.0,
            ));
            fill
        }
        PartialInfillTypes::Cubic => {
            let mut fill = partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                3.0 * settings.layer_width / fill_ratio,
                45.0,
                layer_height / std::f64::consts::SQRT_2,
            );
            fill.append(&mut partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                3.0 * settings.layer_width / fill_ratio,
                45.0 + 120.0,
                layer_height / std::f64::consts::SQRT_2,
            ));
            fill.append(&mut partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                3.0 * settings.layer_width / fill_ratio,
                45.0 + 240.0,
                layer_height / std::f64::consts::SQRT_2,
            ));
            fill
        }
        PartialInfillTypes::Lightning => {
            unreachable!()
        }
    }
}

pub fn spaced_fill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_type: MoveType,
    spacing: f64,
    offset: f64,
) -> Vec<MoveChain> {
    get_monotone_sections(poly)
        .iter()
        .filter_map(|section| {
            let mut current_y = (((section.left_chain[0].y + offset) / spacing).floor()
                - (offset / spacing))
                * spacing;

            let mut orient = true;

            let mut start_point = None;

            let mut left_index = 0;
            let mut right_index = 0;

            let mut moves = vec![];

            loop {
                let mut connect_chain = vec![];
                while left_index < section.left_chain.len()
                    && section.left_chain[left_index].y > current_y
                {
                    if orient {
                        connect_chain.push(section.left_chain[left_index]);
                    }
                    left_index += 1;
                }

                if left_index == section.left_chain.len() {
                    break;
                }

                while right_index < section.right_chain.len()
                    && section.right_chain[right_index].y > current_y
                {
                    if !orient {
                        connect_chain.push(section.right_chain[right_index]);
                    }
                    right_index += 1;
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

                //add moves to connect lines together
                if start_point.is_some() {
                    //Only if not first point
                    let mut y = None;

                    for point in connect_chain {
                        moves.push(Move {
                            end: point,
                            //don''t fill lateral y moves
                            move_type: if y == Some(point.y) {
                                MoveType::Travel
                            } else {
                                fill_type
                            },
                            width: settings.layer_width,
                        });

                        y = Some(point.y);
                    }
                }

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
                        move_type: fill_type,
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
                        move_type: fill_type,
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
            }

            start_point.map(|start_point| MoveChain { start_point, moves })
        })
        .collect::<Vec<_>>()
        .into_iter()
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
