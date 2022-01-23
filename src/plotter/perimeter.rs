use gladius_shared::settings::LayerSettings;
use gladius_shared::types::{Move, MoveChain, MoveType};

use geo::prelude::*;
use geo::*;

use crate::PolygonOperations;
use itertools::Itertools;

pub fn inset_polygon_recursive(
    poly: &MultiPolygon<f64>,
    settings: &LayerSettings,
    outer_perimeter: bool,
    layer_left: usize,
) -> Option<MoveChain> {
    let mut move_chains = vec![];
    let inset_poly = poly.offset_from(-settings.layer_width / 2.0);

    for raw_polygon in inset_poly.0.iter() {
        let polygon = raw_polygon.simplify(&0.01);
        let mut outer_chains = vec![];
        let mut inner_chains = vec![];
        let moves = polygon
            .exterior()
            .0
            .iter()
            .circular_tuple_windows::<(_, _)>()
            .map(|(&_start, &end)| Move {
                end,
                move_type: if outer_perimeter {
                    MoveType::OuterPerimeter
                } else {
                    MoveType::InnerPerimeter
                },
                width: settings.layer_width,
            })
            .collect();

        outer_chains.push(MoveChain {
            start_point: polygon.exterior()[0],
            moves,
        });

        for interior in polygon.interiors() {
            let mut moves = vec![];
            for (&_start, &end) in interior.0.iter().circular_tuple_windows::<(_, _)>() {
                moves.push(Move {
                    end,
                    move_type: if outer_perimeter {
                        MoveType::OuterPerimeter
                    } else {
                        MoveType::InnerPerimeter
                    },
                    width: settings.layer_width,
                });
            }
            outer_chains.push(MoveChain {
                start_point: interior.0[0],
                moves,
            });
        }

        if layer_left != 0 {
            let rec_inset_poly = polygon.offset_from(-settings.layer_width / 2.0);

            for polygon_rec in rec_inset_poly {
                if let Some(mc) = inset_polygon_recursive(
                    &MultiPolygon::from(polygon_rec),
                    settings,
                    false,
                    layer_left - 1,
                ) {
                    inner_chains.push(mc);
                }
            }
        }

        if settings.inner_perimeters_first {
            move_chains.append(&mut inner_chains);
            move_chains.append(&mut outer_chains);
        } else {
            move_chains.append(&mut outer_chains);
            move_chains.append(&mut inner_chains);
        }
    }

    let mut full_moves = vec![];
    move_chains
        .get(0)
        .map(|mc| mc.start_point)
        .map(|starting_point| {
            for mut chain in move_chains {
                full_moves.push(Move {
                    end: chain.start_point,
                    move_type: MoveType::Travel,
                    width: 0.0,
                });
                full_moves.append(&mut chain.moves)
            }

            MoveChain {
                moves: full_moves,
                start_point: starting_point,
            }
        })
}
