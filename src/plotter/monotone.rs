use geo::simplifyvw::SimplifyVWPreserve;
use geo::{Coordinate, Polygon};
use geo_svg::*;
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug)]
pub struct MonotoneSection {
    pub left_chain: Vec<Coordinate<f64>>,
    pub right_chain: Vec<Coordinate<f64>>,
}

#[derive(Debug, PartialEq)]
struct MonotonePoint {
    pos: Coordinate<f64>,
    next: Coordinate<f64>,
    prev: Coordinate<f64>,
    point_type: PointType,
}

impl PartialOrd for MonotonePoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.pos
            .y
            .partial_cmp(&other.pos.y)
            .map(|cmp| cmp.then(self.pos.x.partial_cmp(&other.pos.x).unwrap()))
    }
}

impl Eq for MonotonePoint {}

impl Ord for MonotonePoint {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PointType {
    Start,
    End,
    Merge,
    Split,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Orientation {
    Linear,
    Left,
    Right,
}

/// Divides a Polygon into Y-Monotone sections.
///
/// The sections will only intersect any line perpendicular to the y-axis in two places.
///
/// # Arguments
///
/// * `poly` - the polygon to divide
pub fn get_monotone_sections(poly: &Polygon<f64>) -> Vec<MonotoneSection> {
    //Convert polygon to Monotone points
    //Simplify to remove self intersections
    let mut mono_points = std::iter::once(poly.simplifyvw_preserve(&0.0001).exterior())
        .chain(poly.simplifyvw_preserve(&0.0001).interiors().iter())
        .flat_map(|line_string| {
            line_string
                .0
                .iter()
                .take(line_string.0.len() - 1)
                .circular_tuple_windows::<(&Coordinate<f64>, &Coordinate<f64>, &Coordinate<f64>)>()
                .map(|(&next, &point, &prev)| {
                    // Identify what type of point this is
                    let point_type = if isabove(&point, &prev) && isabove(&point, &next) {
                        if orientation(&prev, &point, &next) != Orientation::Right {
                            PointType::Split
                        } else {
                            PointType::Start
                        }
                    } else if !isabove(&point, &prev) && !isabove(&point, &next) {
                        if orientation(&prev, &point, &next) != Orientation::Right {
                            PointType::Merge
                        } else {
                            PointType::End
                        }
                    } else if isabove(&point, &prev) && !isabove(&point, &next) {
                        PointType::Left
                    } else {
                        PointType::Right
                    };

                    MonotonePoint {
                        pos: point,
                        next,
                        prev,
                        point_type,
                    }
                })
        })
        .collect::<BinaryHeap<MonotonePoint>>();

    let mut sweep_line_storage: Vec<MonotoneSection> = vec![];
    let mut completed_sections: Vec<MonotoneSection> = vec![];

    while let Some(point) = mono_points.pop() {
        match point.point_type {
            //Handle Start Point
            PointType::Start => {
                let new_section = MonotoneSection {
                    left_chain: vec![point.pos, point.prev],
                    right_chain: vec![point.pos, point.next],
                };

                let index = sweep_line_storage
                    .iter()
                    .position(|section| {
                        let right_top = section
                            .right_chain
                            .get(section.right_chain.len() - 2)
                            .unwrap();
                        let right_bot = section.right_chain.last().unwrap();

                        let right_x = point_lerp(right_top, right_bot, point.pos.y).x;
                        point.pos.x < right_x
                    })
                    .unwrap_or(sweep_line_storage.len());
                sweep_line_storage.insert(index, new_section);
            }
            //Handle End Point
            PointType::End => {
                let index = sweep_line_storage
                    .iter()
                    .position(|section| *section.left_chain.last().unwrap() == point.pos)
                    .unwrap_or_else(|| {
                        panic!(
                            "End point must be in the storage {:?} |||| {:?}",
                            point, sweep_line_storage
                        )
                    });

                //The section what was finished should be returned
                let removed_section = sweep_line_storage.remove(index);
                completed_sections.push(removed_section);
            }
            //Handle Left Point
            PointType::Left => {
                let index = sweep_line_storage
                    .iter()
                    .position(|section| *section.left_chain.last().unwrap() == point.pos)
                    .unwrap_or_else(|| panic!("left error {:?} {:?}", point, sweep_line_storage));

                sweep_line_storage[index].left_chain.push(point.prev);
            }
            //Handle Right Point
            PointType::Right => {
                let index = sweep_line_storage
                    .iter()
                    .position(|section| *section.right_chain.last().unwrap() == point.pos)
                    .unwrap_or_else(|| {
                        panic!(
                            "right error {:?}\n {}",
                            point,
                            poly.to_svg()
                                .with_stroke_width(0.01)
                                .with_fill_color(Color::Named("red"))
                                .with_stroke_color(Color::Rgb(200, 0, 100))
                                .with_fill_opacity(0.7)
                        )
                    });

                sweep_line_storage[index].right_chain.push(point.next);
            }

            //Handle Merge Point
            PointType::Merge => {
                let index = sweep_line_storage.iter().position(|section| *section.right_chain.last().unwrap() == point.pos).unwrap_or_else( || panic!("Merge point must be in the storage as the end of a chain{:?} |||| {:?}", point, sweep_line_storage));

                let mut right_section = sweep_line_storage.remove(index + 1);
                let left_section = &mut sweep_line_storage[index];

                assert_eq!(
                    *left_section.right_chain.last().unwrap(),
                    *right_section.left_chain.last().unwrap()
                );

                //The new point generated on the right most edge
                let break_point_low = right_section.right_chain.pop().unwrap();
                let break_point_high = right_section.right_chain.last().unwrap();

                let break_point = point_lerp(break_point_high, &break_point_low, point.pos.y);

                right_section.right_chain.push(break_point);

                completed_sections.push(right_section);

                left_section.right_chain.push(break_point);
                left_section.right_chain.push(break_point_low);
            }

            //Handle Split Point
            PointType::Split => {
                //find the section that will be split up
                let index = sweep_line_storage
                    .iter()
                    .position(|section| {
                        let left_top = section
                            .left_chain
                            .get(section.left_chain.len() - 2)
                            .unwrap();
                        let left_bot = section.left_chain.last().unwrap();
                        let right_top = section
                            .right_chain
                            .get(section.right_chain.len() - 2)
                            .unwrap();
                        let right_bot = section.right_chain.last().unwrap();

                        let left_x = point_lerp(left_top, left_bot, point.pos.y).x;
                        let right_x = point_lerp(right_top, right_bot, point.pos.y).x;

                        point.pos.x > left_x && point.pos.x < right_x
                    })
                    .unwrap_or_else(|| panic!("split error {:?} {:?}", point, sweep_line_storage));

                //will become new left section
                let old_section = sweep_line_storage.get_mut(index).unwrap();

                let break_point_low = old_section.right_chain.pop().unwrap();
                let break_point_high = old_section.right_chain.last().unwrap();

                let break_point = point_lerp(break_point_high, &break_point_low, point.pos.y);

                old_section.right_chain.push(break_point);
                old_section.right_chain.push(point.pos);
                old_section.right_chain.push(point.next);

                let new_right_section = MonotoneSection {
                    left_chain: vec![point.pos, point.prev],
                    right_chain: vec![break_point, break_point_low],
                };
                sweep_line_storage.insert(index + 1, new_right_section);
            }
        }
    }

    completed_sections
}

fn isabove(a: &Coordinate<f64>, b: &Coordinate<f64>) -> bool {
    a.y.partial_cmp(&b.y)
        .map(|cmp| cmp.then(a.x.partial_cmp(&b.x).unwrap()))
        .unwrap()
        == Ordering::Greater
}

fn orientation(p: &Coordinate<f64>, q: &Coordinate<f64>, r: &Coordinate<f64>) -> Orientation {
    let left_val = (q.x - p.x) * (r.y - p.y);
    let right_val = (q.y - p.y) * (r.x - p.x);

    if left_val == right_val {
        Orientation::Linear
    } else if left_val > right_val {
        Orientation::Left
    } else {
        Orientation::Right
    }
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
