use crate::*;
use geo::coordinate_position::{CoordPos, CoordinatePosition};
use geo::euclidean_distance::EuclideanDistance;
use geo::line_intersection::{line_intersection, LineIntersection};
use geo::prelude::*;
use gladius_shared::settings::*;

use rand::seq::SliceRandom;
use rand::thread_rng;

pub fn lightning_infill(slices: &mut Vec<Slice>) {
    let mut lt = LightningForest { trees: vec![] };

    lightning_layer(slices.last_mut().unwrap(), None, &mut lt);

    (1..slices.len()).into_iter().rev().for_each(|q| {
        //todo Fix this, it feels hacky
        if let [ref mut layer, ref mut above, ..] = &mut slices[(q - 1..=q)] {
            lightning_layer(layer, Some(above), &mut lt);
        } else {
            unreachable!()
        }
    });

    for slice in slices {
        slice.remaining_area = MultiPolygon(vec![]);
    }
}

pub fn lightning_layer(
    slice: &mut Slice,
    slice_above: Option<&mut Slice>,
    lightning_forest: &mut LightningForest,
) {
    let spacing = slice.layer_settings.layer_width / slice.layer_settings.infill_percentage;
    let overlap = ((-slice.layer_settings.layer_width / 2.0)
        * (1.0 - slice.layer_settings.infill_perimeter_overlap_percentage))
        + (slice.layer_settings.layer_width / 2.0);
    let inset_amount = slice.layer_settings.layer_height + overlap;

    let unsupported_area = if let Some(area_above) = slice_above.map(|sa| &sa.remaining_area) {
        slice
            .remaining_area
            .difference_with(area_above)
            .offset_from(-(inset_amount))
    } else {
        slice.remaining_area.offset_from(-(inset_amount))
    };

    let infill_area = slice.remaining_area.clone().offset_from(-overlap);

    let (min_x, max_x, min_y, max_y) = unsupported_area
        .iter()
        .flat_map(|poly| poly.exterior().0.iter())
        .fold(
            (
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ),
            |a, b| (a.0.min(b.x), a.1.max(b.x), a.2.min(b.y), a.3.max(b.y)),
        );

    let h_spacing = spacing;
    let v_spacing = h_spacing * (3.0_f64).sqrt() / 2.0;

    let fragments = lightning_forest.reconnect_to_polygon_and_trim(&infill_area);

    let mut points: Vec<_> = ((min_x / h_spacing) as usize..=(max_x / h_spacing) as usize + 1)
        .cartesian_product((min_y / v_spacing) as usize..=(max_y / v_spacing) as usize + 1)
        .map(|(x, y)| {
            if y % 2 == 0 {
                (x as f64 * h_spacing, y as f64 * v_spacing)
            } else {
                ((x as f64 - 0.5) * h_spacing, y as f64 * v_spacing)
            }
        })
        .map(|(x, y)| Coordinate { x, y })
        .filter(|coord| unsupported_area.contains(coord))
        .map(|coord| LightningNode {
            children: vec![],
            location: coord,
        })
        .chain(fragments.into_iter())
        .filter_map(|node| {
            if let Closest::SinglePoint(closest_point) =
                infill_area.closest_point(&node.location.into())
            {
                let closest_coordinate: Coordinate<f64> = closest_point.into();
                let distance: f64 = node.location.euclidean_distance(&closest_coordinate);
                Some((node, distance, closest_coordinate))
            } else {
                None
            }
        })
        .collect();

    if !points.is_empty() {
        //shuffle so same distance points are random
        points.shuffle(&mut thread_rng());

        points.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        for (node, _distance, closet) in points {
            lightning_forest.add_node_to_tree(node, &closet, inset_amount)
        }
    }

    lightning_forest.shorten_and_straighten(&slice.layer_settings);

    let width = slice.layer_settings.layer_width;
    slice.chains.extend(
        lightning_forest
            .trees
            .iter()
            .flat_map(|tree| tree.get_move_chains(width).into_iter()),
    )
}

pub enum StraightenResponse {
    Remove { remaining_len: f64 },
    Replace(LightningNode),
    DoNothing,
}

pub struct LightningNode {
    children: Vec<LightningNode>,
    location: Coordinate<f64>,
}

impl LightningNode {
    fn add_point_to_tree(&mut self, node: LightningNode) {
        let self_dist = self.location.euclidean_distance(&node.location);

        if let Some((index, closest)) = self
            .children
            .iter()
            .enumerate()
            .map(|(index, child)| (index, child.get_closest_child(&node.location)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        {
            if closest < self_dist {
                self.children
                    .get_mut(index)
                    .unwrap()
                    .add_point_to_tree(node);
                return;
            }
        }

        self.children.push(node);
    }

    fn shorten_and_straighten(
        &mut self,
        parent_location: Coordinate<f64>,
        settings: &LayerSettings,
    ) -> StraightenResponse {
        let l = self.location;
        let max_move = settings.layer_width / 2.0;
        let mut shorten_amount = max_move;

        //reverse to make removals safe
        self.children = self
            .children
            .drain(..)
            .filter_map(|mut child| {
                let reponse = child.shorten_and_straighten(l, settings);
                match reponse {
                    StraightenResponse::Remove { remaining_len } => {
                        shorten_amount = remaining_len;
                        None
                    }
                    StraightenResponse::Replace(new_node) => Some(new_node),
                    StraightenResponse::DoNothing => Some(child),
                }
            })
            .collect();

        if self.children.is_empty() {
            //No children so shorten directly
            let line_len = self.location.euclidean_distance(&parent_location);

            if line_len > shorten_amount {
                let dx = self.location.x - parent_location.x;
                let dy = self.location.y - parent_location.y;

                let newdx = dx * ((line_len - shorten_amount) / line_len);
                let newdy = dy * ((line_len - shorten_amount) / line_len);

                let newx = parent_location.x + newdx;
                let newy = parent_location.y + newdy;

                self.location = Coordinate { x: newx, y: newy };

                StraightenResponse::DoNothing
            } else {
                StraightenResponse::Remove {
                    remaining_len: shorten_amount - line_len,
                }
            }
        } else if self.children.len() == 1 {
            let l = self.location;
            let child_location = self.children[0].location;
            if l == parent_location {
                //dont straighten the starts of trees
                StraightenResponse::DoNothing
            } else {
                let pl_dist = l.euclidean_distance(&parent_location);
                let lc_dist = l.euclidean_distance(&child_location);
                let pl_ratio = pl_dist / (pl_dist + lc_dist);
                let midpoint = (child_location * (1.0 - pl_ratio)) + (parent_location * pl_ratio);

                let line_len = l.euclidean_distance(&midpoint);
                if line_len > shorten_amount {
                    let dx = l.x - midpoint.x;
                    let dy = l.y - midpoint.y;

                    let newdx = dx * ((line_len - shorten_amount) / line_len);
                    let newdy = dy * ((line_len - shorten_amount) / line_len);

                    let newx = midpoint.x + newdx;
                    let newy = midpoint.y + newdy;

                    self.location = Coordinate { x: newx, y: newy };

                    StraightenResponse::DoNothing
                } else {
                    let child = self.children.remove(0);
                    StraightenResponse::Replace(child)
                }
            }
        } else {
            StraightenResponse::DoNothing
        }
    }

    fn get_closest_child(&self, point: &Coordinate<f64>) -> f64 {
        let min_dist = self.location.euclidean_distance(point)
            - if self.children.len() > 0 && self.children.len() < 4 {
                (2.0/* - self.children.len() as f64*/) * 0.45 / 2.0
            } else {
                0.0
            };
        let min_child = self
            .children
            .iter()
            .map(|child| child.get_closest_child(point))
            .min_by(|a, b| a.partial_cmp(b).unwrap());

        if let Some(min_child_dist) = min_child {
            min_dist.min(min_child_dist)
        } else {
            min_dist
        }
    }

    fn get_move_chains(&self, width: f64) -> Vec<MoveChain> {
        self.children
            .iter()
            .flat_map(|child| {
                let mut chains = child.get_move_chains(width);

                if !chains.is_empty() {
                    let first_chain = chains.get_mut(0).unwrap();
                    first_chain.moves.push(Move {
                        end: self.location,
                        width,
                        move_type: MoveType::Infill,
                    });
                } else {
                    chains.push(MoveChain {
                        moves: vec![Move {
                            end: self.location,
                            width,
                            move_type: MoveType::Infill,
                        }],
                        start_point: child.location,
                    })
                }
                chains.into_iter()
            })
            .collect()
    }

    fn trim_for_polygon_inside(&mut self, polygon: &MultiPolygon<f64>) -> Vec<LightningNode> {
        let l = self.location;

        self.children
            .iter_mut()
            .flat_map(|child| {
                if polygon.contains(&child.location) {
                    child.trim_for_polygon_inside(polygon)
                } else {
                    let intersection = get_closest_intersection_point_on_polygon(
                        Line {
                            start: l,
                            end: child.location,
                        },
                        polygon,
                    )
                    .unwrap();

                    let new_child = LightningNode {
                        children: vec![],
                        location: intersection,
                    };
                    let old_child = std::mem::replace(child, new_child);

                    old_child.trim_for_polygon_outside(polygon)
                }
                .into_iter()
            })
            .collect()
    }

    fn trim_for_polygon_outside(self, polygon: &MultiPolygon<f64>) -> Vec<LightningNode> {
        let l = self.location;

        self.children
            .into_iter()
            .flat_map(|child| {
                if polygon.contains(&child.location) {
                    let intersection = get_closest_intersection_point_on_polygon(
                        Line {
                            start: child.location,
                            end: l,
                        },
                        polygon,
                    )
                    .unwrap();

                    let mut new_node = LightningNode {
                        children: vec![child],
                        location: intersection,
                    };
                    let mut frags = new_node.trim_for_polygon_inside(polygon);

                    frags.push(new_node);

                    frags
                } else {
                    child.trim_for_polygon_outside(polygon)
                }
                .into_iter()
            })
            .collect()
    }
}

pub struct LightningForest {
    trees: Vec<LightningNode>,
}

impl LightningForest {
    fn add_node_to_tree(
        &mut self,
        node: LightningNode,
        closest_point_on_polygon: &Coordinate<f64>,
        min_distance: f64,
    ) {
        let poly_dist = node.location.euclidean_distance(closest_point_on_polygon);

        if poly_dist < min_distance {
            //connect to polygon if below min distance
            //handle minor wall movements
            self.trees.push(LightningNode {
                children: vec![node],
                location: *closest_point_on_polygon,
            });

            return;
        }
        if let Some((index, closest)) = self
            .trees
            .par_iter()
            .enumerate()
            .map(|(index, child)| (index, child.get_closest_child(&node.location)))
            .filter(|(_index, dist)| *dist < poly_dist)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        {
            if closest < poly_dist {
                self.trees.get_mut(index).unwrap().add_point_to_tree(node);
                return;
            }
        }

        self.trees.push(LightningNode {
            children: vec![node],
            location: *closest_point_on_polygon,
        });
    }

    fn reconnect_to_polygon_and_trim(&mut self, polygon: &MultiPolygon<f64>) -> Vec<LightningNode> {
        let mut fragments = vec![];
        let mut new_trees = vec![];

        self.trees.drain(..).for_each(|mut tree| {
            match polygon.coordinate_position(&tree.location) {
                CoordPos::OnBoundary => {
                    new_trees.extend(tree.trim_for_polygon_inside(polygon).into_iter());
                    new_trees.push(tree)
                }
                CoordPos::Outside => {
                    //new_trees.extend(tree.children.into_iter().map(|child| child.trim_for_polygon_outside_to_inside(l,polygon).into_iter()).flatten())
                    new_trees.extend(tree.trim_for_polygon_outside(polygon).into_iter());
                }
                CoordPos::Inside => {
                    new_trees.extend(tree.trim_for_polygon_inside(polygon).into_iter());
                    fragments.push(tree);
                }
            }
        });

        self.trees = new_trees;

        fragments
    }

    fn shorten_and_straighten(&mut self, settings: &LayerSettings) {
        for index in (0..self.trees.len()).rev() {
            let tree_l = self.trees.get(index).unwrap().location;

            let reponse = self
                .trees
                .get_mut(index)
                .unwrap()
                .shorten_and_straighten(tree_l, settings);
            match reponse {
                StraightenResponse::Remove { .. } => {
                    self.trees.remove(index);
                }
                StraightenResponse::Replace(..) => {
                    unreachable!()
                }
                StraightenResponse::DoNothing => {}
            }
        }
    }
}

fn get_closest_intersection_point_on_polygon(
    line: Line<f64>,
    poly: &MultiPolygon<f64>,
) -> Option<Coordinate<f64>> {
    poly.iter()
        .flat_map(|poly| {
            std::iter::once(poly.exterior())
                .chain(poly.interiors())
                .flat_map(|chain| chain.lines())
        })
        .filter_map(|poly_line| {
            line_intersection(poly_line, line).map(|intersection| match intersection {
                LineIntersection::SinglePoint { intersection, .. } => intersection,
                LineIntersection::Collinear { intersection } => intersection.end,
            })
        })
        .map(|coord| (coord, coord.euclidean_distance(&line.start) as f64))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(c, _d)| c)
}
