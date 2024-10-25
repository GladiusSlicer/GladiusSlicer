use crate::plotter::monotone::get_monotone_sections;
use gladius_shared::settings::LayerSettings;
use gladius_shared::types::{Move, MoveChain, MoveType, PartialInfillTypes, SolidInfillTypes};

use crate::utils::point_y_lerp;
use crate::PolygonOperations;
use geo::prelude::*;
use geo::{Coord, Point, Polygon};

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
    let rotate_poly = poly.rotate_around_point(angle, Point(Coord::zero()));

    let mut new_moves: Vec<MoveChain> = rotate_poly
        .offset_from(
            ((-settings.extrusion_width.interior_inner_perimeter / 2.0)
                * (1.0 - settings.infill_perimeter_overlap_percentage))
                + (settings.extrusion_width.interior_inner_perimeter / 2.0),
        )
        .iter()
        .flat_map(|polygon| {
            spaced_fill_polygon(
                polygon,
                settings,
                fill_type,
                settings
                    .extrusion_width
                    .get_value_for_movement_type(&fill_type),
                0.0,
            )
        })
        .collect();

    for chain in &mut new_moves {
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
    let rotate_poly = poly.rotate_around_point(angle, Point(Coord::zero()));

    let mut new_moves: Vec<MoveChain> = rotate_poly
        .offset_from(
            ((-settings.extrusion_width.interior_inner_perimeter / 2.0)
                * (1.0 - settings.infill_perimeter_overlap_percentage))
                + (settings.extrusion_width.interior_inner_perimeter / 2.0),
        )
        .iter()
        .flat_map(|polygon| spaced_fill_polygon(polygon, settings, fill_type, spacing, offset))
        .collect();

    for chain in &mut new_moves {
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
    let rotate_poly = poly.rotate_around_point(angle, Point(Coord::zero()));

    let mut new_moves: Vec<MoveChain> = rotate_poly
        .offset_from(-settings.extrusion_width.interior_surface_perimeter / 2.0)
        .iter()
        .flat_map(|polygon| spaced_fill_polygon(polygon, settings, fill_type, spacing, offset))
        .collect();

    for chain in &mut new_moves {
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
    match settings.solid_infill_type {
        SolidInfillTypes::Rectilinear => {
            //120 degrees between layers
            let angle = 45.0 + (120_f64) * layer_count as f64;

            linear_fill_polygon(poly, settings, fill_type, angle)
        }

        SolidInfillTypes::RectilinearCustom(degrees_per_angle) => {
            let angle = 45.0 + (degrees_per_angle) * layer_count as f64;

            linear_fill_polygon(poly, settings, fill_type, angle)
        }
    }
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
            settings.extrusion_width.infill / fill_ratio,
            0.0,
            0.0,
        ),
        PartialInfillTypes::Rectilinear => {
            let mut fill = partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                2.0 * settings.extrusion_width.infill / fill_ratio,
                45.0,
                0.0,
            );
            fill.append(&mut partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                2.0 * settings.extrusion_width.infill / fill_ratio,
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
                3.0 * settings.extrusion_width.infill / fill_ratio,
                45.0,
                0.0,
            );
            fill.append(&mut partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                3.0 * settings.extrusion_width.infill / fill_ratio,
                45.0 + 60.0,
                0.0,
            ));
            fill.append(&mut partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                3.0 * settings.extrusion_width.infill / fill_ratio,
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
                3.0 * settings.extrusion_width.infill / fill_ratio,
                45.0,
                layer_height / std::f64::consts::SQRT_2,
            );
            fill.append(&mut partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                3.0 * settings.extrusion_width.infill / fill_ratio,
                45.0 + 120.0,
                layer_height / std::f64::consts::SQRT_2,
            ));
            fill.append(&mut partial_linear_fill_polygon(
                poly,
                settings,
                MoveType::Infill,
                3.0 * settings.extrusion_width.infill / fill_ratio,
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

                let left_point = point_y_lerp(&left_top, &left_bot, current_y);
                let right_point = point_y_lerp(&right_top, &right_bot, current_y);

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
                            width: settings
                                .extrusion_width
                                .get_value_for_movement_type(&fill_type),
                        });

                        y = Some(point.y);
                    }
                }

                start_point = start_point.or(Some(Coord {
                    x: left_point.x,
                    y: current_y,
                }));

                if orient {
                    moves.push(Move {
                        end: Coord {
                            x: left_point.x,
                            y: current_y,
                        },
                        move_type: fill_type,
                        width: settings
                            .extrusion_width
                            .get_value_for_movement_type(&fill_type),
                    });

                    moves.push(Move {
                        end: Coord {
                            x: right_point.x,
                            y: current_y,
                        },
                        move_type: fill_type,
                        width: settings
                            .extrusion_width
                            .get_value_for_movement_type(&fill_type),
                    });
                } else {
                    moves.push(Move {
                        end: Coord {
                            x: right_point.x,
                            y: current_y,
                        },
                        move_type: fill_type,
                        width: settings
                            .extrusion_width
                            .get_value_for_movement_type(&fill_type),
                    });

                    moves.push(Move {
                        end: Coord {
                            x: left_point.x,
                            y: current_y,
                        },
                        move_type: fill_type,
                        width: settings
                            .extrusion_width
                            .get_value_for_movement_type(&fill_type),
                    });
                }

                orient = !orient;
                current_y -= spacing;
            }

            start_point.map(|start_point| MoveChain {
                start_point,
                moves,
                is_loop: false,
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .collect()
}
