use geo::coordinate_position::{CoordinatePosition, CoordPos};
use geo::euclidean_distance::EuclideanDistance;
use geo::line_intersection::{line_intersection, LineIntersection};
use geo::prelude::*;
use itertools::chain;
use nalgebra::{distance, min};
use crate::*;
use crate::settings::*;

use rand::thread_rng;
use rand::seq::SliceRandom;

pub fn lightning_infill(slices : &mut Vec<Slice> ) {

    let mut lt = LightningForest{trees: vec![]};


    lightning_layer(slices.last_mut().unwrap(),None,&mut lt);

    (1..slices.len()).into_iter().rev().for_each(|q| {
        //todo Fix this, it feels hacky
        if let [ref mut layer, ref mut above, ..] = &mut slices[(q-1 ..=q)] {
            lightning_layer(layer, Some(above), &mut lt);
        } else {
            unreachable!()
        }
    });

    for slice in slices{
        slice.remaining_area = MultiPolygon(vec![]);
    }
}

pub fn lightning_layer(slice: &mut Slice, slice_above: Option<&mut Slice> , lightning_forest: &mut LightningForest) {
    let mut unsupported_area = if let Some(area_above) = slice_above.map(|sa| &sa.remaining_area) {
        slice.remaining_area.difference_with(area_above)
    } else {
        slice.remaining_area.clone()
    };
    let (min_x, max_x, min_y, max_y) =
        unsupported_area.iter().map(|poly| poly.exterior().0.iter()).flatten().fold(
            (
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ),
            |a, b| {
                (
                    a.0.min(b.x),
                    a.1.max(b.x),
                    a.2.min(b.y),
                    a.3.max(b.y),
                )
            },
        );

    let h_spacing = slice.layer_settings.layer_width /  slice.layer_settings.infill_percentage;
    let v_spacing =h_spacing * (3.0_f64).sqrt() / 2.0;


    let fragments = lightning_forest.reconnect_to_polygon_and_trim(&slice.remaining_area);

    let mut points: Vec<_> = ((min_x / h_spacing) as usize..=(max_x / h_spacing) as usize + 1)
        .cartesian_product(((min_y / v_spacing) as usize..=(max_y / v_spacing) as usize + 1))
        .map(|(x, y)| {
            if y % 2 ==0 {
                (x as f64 * h_spacing, y as f64 * v_spacing)
            }
            else{
                ((x as  f64 -0.5) * h_spacing, y as f64 * v_spacing)
            }
        })
        .map(|(x, y)| Coordinate { x, y })
        .filter(|coord| {
            unsupported_area.contains(coord)
        })
        .map(|coord| LightningNode{children: vec![], location: coord})
        .chain(fragments.into_iter())
        .filter_map(|node| {
            if let Closest::SinglePoint(closest_point) = slice.remaining_area.closest_point(&node.location.into()) {
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

        for (node,distance,closet) in points{
            lightning_forest.add_node_to_tree(node,&closet)
        }
    }

    let width = slice.layer_settings.layer_width;
    slice.chains.extend(lightning_forest.trees.iter()
        .map(|tree|{
            tree.get_move_chains(width ).into_iter()
        })
        .flatten()
    )
}

pub struct LightningNode{
    children: Vec<LightningNode>,
    location :  Coordinate<f64>
}

impl LightningNode{
    fn add_point_to_tree(&mut self, node: LightningNode) {
        let self_dist = self.location.euclidean_distance(&node.location);

        if let Some((index,closest) )= self.children
            .iter()
            .enumerate()
            .map(|(index,child)| (index,child.get_closest_child(&node.location)))
            .min_by(|a,b| a.1.partial_cmp(&b.1).unwrap()){
            if closest < self_dist{
                self.children.get_mut(index).unwrap().add_point_to_tree(node);
                return;
            }
        }

        self.children.push(node);
    }


    fn get_closest_child(&self, point: &Coordinate<f64>) -> f64{

        let min_dist = self.location.euclidean_distance(point);
        let  min_child = self.children
                .iter()
                .map(|child|{
                    child.get_closest_child(point)
                })
                .min_by(|a,b| a.partial_cmp(&b).unwrap());

        if let Some(min_child_dist) = min_child{
            min_dist.min(min_child_dist)
        }
        else{
            min_dist
        }
    }

    fn get_move_chains(&self,width: f64) -> Vec<MoveChain>{

        self.children
            .iter()
            .map(|child|{

                let mut chains  = child.get_move_chains(width);

                if !chains.is_empty(){
                    let first_chain = chains.get_mut(0).unwrap();
                    first_chain.moves.push(Move{
                        end: self.location,
                        width,
                        move_type: MoveType::Infill
                    });
                }
                else{
                    chains.push(MoveChain{moves: vec![Move{
                        end: self.location,
                        width,
                        move_type: MoveType::Infill
                    }],start_point: child.location})
                }
                chains.into_iter()
            })
            .flatten()
            .collect()
    }

    fn trim_for_polygon_inside(&mut self, polygon: &MultiPolygon<f64> ) -> Vec<LightningNode> {

        let l = self.location;

        self.children
            .iter_mut()
            .map(|mut child| {

                if polygon.contains(&child.location) {

                    child.trim_for_polygon_inside(polygon)

                } else {
                    let intersection = get_closest_intersection_point_on_polygon(Line { start: l , end: child.location}, &polygon).unwrap();

                    let new_child = LightningNode { children: vec![], location: intersection };
                    let old_child = std::mem::replace(child,new_child );

                    old_child.trim_for_polygon_outside( polygon)

                }.into_iter()
            })
            .flatten().collect()
    }

    fn trim_for_polygon_outside(self, polygon: &MultiPolygon<f64> ) -> Vec<LightningNode> {

        let l = self.location;

        self.children
            .into_iter()
            .map(|mut child| {

                if polygon.contains(&child.location) {

                    let intersection = get_closest_intersection_point_on_polygon(Line { start: child.location , end: l}, &polygon).unwrap();

                    let mut new_node = LightningNode { children: vec![child], location: intersection };
                    let mut frags = new_node.trim_for_polygon_inside(polygon);

                    frags.push(new_node);

                    frags

                } else {
                    child.trim_for_polygon_outside( polygon)

                }.into_iter()
            })
            .flatten().collect()
    }

}


pub struct LightningForest{
    trees: Vec<LightningNode>,
}

impl LightningForest {

    fn add_node_to_tree(&mut self, mut node: LightningNode, closest_point_on_polygon: &Coordinate<f64> ){
        let poly_dist = node.location.euclidean_distance(closest_point_on_polygon);


        if let Some((index,closest) )= self.trees
            .iter()
            .enumerate()
            .map(|(index,child)| (index,child.get_closest_child(&node.location)))
            .filter(|(index,dist)| *dist < poly_dist)
            .min_by(|a,b| a.1.partial_cmp(&b.1).unwrap()){
                if closest < poly_dist{
                    self.trees.get_mut(index).unwrap().add_point_to_tree(node);
                    return;
                }
            }

        self.trees.push(LightningNode{children: vec![node], location: *closest_point_on_polygon});

    }

    fn reconnect_to_polygon_and_trim(&mut self,  polygon: &MultiPolygon<f64> ) -> Vec<LightningNode>{

        let mut fragments = vec![];
        let mut new_trees = vec![];

        self.trees
            .drain(..)
            .for_each(|mut tree|{
                match polygon.coordinate_position(&tree.location) {
                    CoordPos::OnBoundary =>{
                        new_trees.extend(tree.trim_for_polygon_inside(polygon).into_iter());
                        new_trees.push(tree)
                    },CoordPos::Outside   =>{
                        //new_trees.extend(tree.children.into_iter().map(|child| child.trim_for_polygon_outside_to_inside(l,polygon).into_iter()).flatten())
                        new_trees.extend(tree.trim_for_polygon_outside(polygon).into_iter());

                    },CoordPos::Inside =>{
                        new_trees.extend(tree.trim_for_polygon_inside(polygon).into_iter());
                        fragments.push(tree);
                    },
                }
            });

        self.trees = new_trees;

        fragments
    }


}

fn get_closest_intersection_point_on_polygon(line:Line<f64>, poly: &MultiPolygon<f64>) -> Option<Coordinate<f64>>{
    poly.iter()
        .map(|poly| {
            std::iter::once(poly.exterior())
                .chain(poly.interiors())
                .map(|chain| chain.lines())
                .flatten()
        })
        .flatten()

        .filter_map(|poly_line| line_intersection(poly_line,line).map(|intersection|{
            match  intersection {
                LineIntersection::SinglePoint {intersection,..} => intersection,
                LineIntersection::Collinear {intersection} => intersection.end,
            }
        }))
        .map(|coord| (coord,coord.euclidean_distance(&line.start ) as f64))
        .min_by(|a,b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(c,d)| c)

}