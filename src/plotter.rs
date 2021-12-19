use crate::settings::LayerSettings;
use crate::types::{Command, Move, MoveChain, MoveType};
use geo::prelude::*;
use geo::*;
use geo_clipper::*;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use std::iter::FromIterator;
use geo::coordinate_position::CoordPos;
use geo::coordinate_position::CoordinatePosition;
use rayon::prelude::*;
use geo_svg::{ToSvgStr, ToSvg, Color};

pub struct Slice {
    MainPolygon: MultiPolygon<f64>,
    remaining_area: MultiPolygon<f64>,
    solid_infill: Option<MultiPolygon<f64>>,
    normal_infill: Option<MultiPolygon<f64>>,
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
        //Create the outer shells
        for _ in 0..number_of_perimeters {
            let (m, mut new_chains) = inset_polygon(&self.remaining_area, settings);
            self.remaining_area = m;
            self.chains.append(&mut new_chains);
        }
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

                if let Some(mut chain) = new_moves {
                    chain.rotate(-angle.to_radians());
                    self.chains.push(chain);
                }
            } else {
                let new_moves = partial_fill_polygon(&poly, settings, settings.infill_percentage);

                if let Some(chain) = new_moves {
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

        self.chains.append(
            &mut solid_area
                .0
                .par_iter()
                .filter_map(|poly| {
                    let angle = 45.0 + (120_f64) * layer_count as f64;

                    let rotate_poly = poly.rotate_around_point(angle, Point(Coordinate::zero()));

                    solid_fill_polygon(&rotate_poly, settings, MoveType::SolidInfill).map(
                        |mut chain| {
                            chain.rotate(-angle.to_radians());
                            chain
                        },
                    )
                })
                .collect(),
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

        self.chains.append(
            &mut solid_area
                .0
                .par_iter()
                .filter_map(|poly| {

                    let unsupported_area :MultiPolygon<f64 >= poly.difference(layer_below,100000.0);
                    let mut angle = get_optimal_bridge_angle(poly,&unsupported_area);

                    if angle < 0.0{
                        angle += 180.0;
                    }
                    //println!("angle {}", angle);

                    let rotate_poly = poly.rotate_around_point(angle, Point(Coordinate::zero()));

                    solid_fill_polygon(&rotate_poly, settings, MoveType::Bridging).map(
                        |mut chain| {
                            chain.rotate(-angle.to_radians());
                            chain
                        },
                    )
                })
                .collect(),
        );

        self.remaining_area = self.remaining_area.difference(&solid_area, 100000.0)
    }

    pub fn slice_into_commands(
        &mut self,
        settings: &LayerSettings,
        commands: &mut Vec<Command>,
        layer_thickness: f64,
    ) {
        //Order Chains for fastest print
        if !self.chains.is_empty() {
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

            let mut full_moves = vec![];
            let starting_point = ordered_chains[0].start_point;
            for mut chain in ordered_chains {
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
}

fn inset_polygon(
    poly: &MultiPolygon<f64>,
    settings: &LayerSettings,
) -> (MultiPolygon<f64>, Vec<MoveChain>) {
    let mut move_chains = vec![];
    let inset_poly = poly.offset(
        -settings.layer_width / 2.0,
        JoinType::Square,
        EndType::ClosedPolygon,
        100000000.0,
    );

    for polygon in inset_poly.0.iter() {
        let moves = polygon
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

        move_chains.push(MoveChain {
            start_point: polygon.exterior()[0],
            moves,
        });

        for interior in polygon.interiors() {
            let mut moves = vec![];
            for (&_start, &end) in interior.0.iter().circular_tuple_windows::<(_, _)>() {
                moves.push(Move {
                    end,
                    move_type: MoveType::OuterPerimeter,
                    width: settings.layer_width,
                });
            }
            move_chains.push(MoveChain {
                start_point: interior.0[0],
                moves,
            });
        }
    }

    (
        inset_poly.offset(
            -settings.layer_width / 2.0,
            JoinType::Square,
            EndType::ClosedPolygon,
            100000000.0,
        ),
        move_chains,
    )
}

fn solid_fill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_type: MoveType,
) -> Option<MoveChain> {
    let mut moves = vec![];

    let mut lines: Vec<(Coordinate<f64>, Coordinate<f64>)> = poly
        .exterior()
        .0
        .iter()
        .copied()
        .circular_tuple_windows::<(_, _)>()
        .collect();

    for interior in poly.interiors() {
        let mut new_lines = interior
            .0
            .iter()
            .copied()
            .circular_tuple_windows::<(_, _)>()
            .collect();
        lines.append(&mut new_lines);
    }

    for line in lines.iter_mut() {
        *line = if line.0.y < line.1.y {
            *line
        } else {
            (line.1, line.0)
        };
    }

    lines.sort_by(|a, b| b.0.y.partial_cmp(&a.0.y).unwrap());

    let mut current_y = lines[lines.len() - 1].0.y + settings.layer_width / 2.0;

    let mut current_lines = Vec::new();

    let mut orient = false;

    let mut start_point = None;

    let mut line_change;

    while !lines.is_empty() || !current_lines.is_empty() {
        line_change = false;
        while !lines.is_empty() && lines[lines.len() - 1].0.y < current_y {
            current_lines.push(lines.pop().unwrap());
            line_change = true;
        }

        current_lines.retain(|(_s, e)| e.y > current_y);

        if current_lines.is_empty() {
            break;
        }

        //current_lines.sort_by(|a,b| b.0.x.partial_cmp(&x.0.y).unwrap().then(b.1.x.partial_cmp(&a.1.x).unwrap()) )

        let mut points = current_lines
            .iter()
            .map(|(start, end)| {
                ((current_y - start.y) * ((end.x - start.x) / (end.y - start.y))) + start.x
            })
            .collect::<Vec<_>>();

        points.sort_by(|a, b| a.partial_cmp(b).unwrap());

        start_point = start_point.or(Some(Coordinate {
            x: points[0],
            y: current_y,
        }));

        moves.push(Move {
            end: Coordinate {
                x: if orient {
                    *points.first().unwrap() + settings.layer_width / 2.0
                } else {
                    *points.last().unwrap() - settings.layer_width / 2.0
                },
                y: current_y,
            },
            move_type: if line_change {
                MoveType::Travel
            } else {
                fill_type
            },
            width: settings.layer_width,
        });

        if orient {
            for (start, end) in points.iter().tuples::<(_, _)>() {
                moves.push(Move {
                    end: Coordinate {
                        x: *start + settings.layer_width / 2.0,
                        y: current_y,
                    },
                    move_type: MoveType::Travel,
                    width: settings.layer_width,
                });

                moves.push(Move {
                    end: Coordinate {
                        x: *end - settings.layer_width / 2.0,
                        y: current_y,
                    },
                    move_type: fill_type,
                    width: settings.layer_width,
                });
            }
        } else {
            for (start, end) in points.iter().rev().tuples::<(_, _)>() {
                moves.push(Move {
                    end: Coordinate {
                        x: *start - settings.layer_width / 2.0,
                        y: current_y,
                    },
                    move_type: MoveType::Travel,
                    width: settings.layer_width,
                });

                moves.push(Move {
                    end: Coordinate {
                        x: *end + settings.layer_width / 2.0,
                        y: current_y,
                    },
                    move_type: fill_type,
                    width: settings.layer_width,
                });
            }
        }

        orient = !orient;
        current_y += settings.layer_width;
    }

    start_point.map(|start_point| MoveChain { start_point, moves })
}

fn partial_fill_polygon(
    poly: &Polygon<f64>,
    settings: &LayerSettings,
    fill_ratio: f64,
) -> Option<MoveChain> {
    let mut moves = vec![];

    let mut lines: Vec<(Coordinate<f64>, Coordinate<f64>)> = poly
        .exterior()
        .0
        .iter()
        .copied()
        .circular_tuple_windows::<(_, _)>()
        .collect();

    for interior in poly.interiors() {
        let mut new_lines = interior
            .0
            .iter()
            .copied()
            .circular_tuple_windows::<(_, _)>()
            .collect();
        lines.append(&mut new_lines);
    }

    for line in lines.iter_mut() {
        *line = if line.0.y < line.1.y {
            *line
        } else {
            (line.1, line.0)
        };
    }

    lines.sort_by(|a, b| b.0.y.partial_cmp(&a.0.y).unwrap());

    let distance = settings.layer_width / fill_ratio;

    let mut current_y = (lines[lines.len() - 1].0.y / distance).ceil() * distance;

    let mut current_lines = Vec::new();

    let mut orient = false;

    let mut start_point = None;

    let mut line_change;

    let distance = settings.layer_width / fill_ratio;

    while !lines.is_empty() || !current_lines.is_empty() {
        line_change = false;
        while !lines.is_empty() && lines[lines.len() - 1].0.y < current_y {
            current_lines.push(lines.pop().unwrap());
            line_change = true;
        }

        current_lines.retain(|(_s, e)| e.y > current_y);

        if current_lines.is_empty() {
            break;
        }

        //current_lines.sort_by(|a,b| b.0.x.partial_cmp(&x.0.y).unwrap().then(b.1.x.partial_cmp(&a.1.x).unwrap()) )

        let mut points = current_lines
            .iter()
            .map(|(start, end)| {
                ((current_y - start.y) * ((end.x - start.x) / (end.y - start.y))) + start.x
            })
            .collect::<Vec<_>>();

        points.sort_by(|a, b| a.partial_cmp(b).unwrap());

        start_point = start_point.or(Some(Coordinate {
            x: points[0],
            y: current_y,
        }));

        moves.push(Move {
            end: Coordinate {
                x: if orient {
                    *points.first().unwrap() + settings.layer_width / 2.0
                } else {
                    *points.last().unwrap() - settings.layer_width / 2.0
                },
                y: current_y,
            },
            move_type: if line_change {
                MoveType::Travel
            } else {
                MoveType::Infill
            },
            width: settings.layer_width,
        });

        if orient {
            for (start, end) in points.iter().tuples::<(_, _)>() {
                moves.push(Move {
                    end: Coordinate {
                        x: *start + settings.layer_width / 2.0,
                        y: current_y,
                    },
                    move_type: MoveType::Travel,
                    width: settings.layer_width,
                });

                moves.push(Move {
                    end: Coordinate {
                        x: *end - settings.layer_width / 2.0,
                        y: current_y,
                    },
                    move_type: MoveType::Infill,
                    width: settings.layer_width,
                });
            }
        } else {
            for (start, end) in points.iter().rev().tuples::<(_, _)>() {
                moves.push(Move {
                    end: Coordinate {
                        x: *start - settings.layer_width / 2.0,
                        y: current_y,
                    },
                    move_type: MoveType::Travel,
                    width: settings.layer_width,
                });

                moves.push(Move {
                    end: Coordinate {
                        x: *end + settings.layer_width / 2.0,
                        y: current_y,
                    },
                    move_type: MoveType::Infill,
                    width: settings.layer_width,
                });
            }
        }

        orient = !orient;
        current_y += distance;
    }

    start_point.map(|start_point| MoveChain { start_point, moves })
}

fn get_optimal_bridge_angle(fill_area: &Polygon<f64>, unsupported_area: &MultiPolygon<f64>) ->f64
{
    let archor_area =  fill_area.difference(unsupported_area,100000.0)
        .offset(-0.001,JoinType::Square,EndType::ClosedPolygon,100000.0)
        .offset(0.01,JoinType::Square,EndType::ClosedPolygon,100000.0);

    let svg = unsupported_area.to_svg()
        .with_stroke_width(0.001)
        .with_fill_color(Color::Named("red"))
        .with_stroke_color(Color::Rgb(200, 0, 100))
        .with_fill_opacity(0.5)
        .and(archor_area.to_svg()
            .with_stroke_width(0.001)
            .with_fill_color(Color::Named("blue"))
            .with_stroke_color(Color::Rgb(100, 0, 200))
            .with_fill_opacity(0.5)

        );

    println!("fill area {}", svg);
    let unsuported_lines :Vec<_>= unsupported_area.iter()
        .map(|poly| std::iter::once(poly.exterior() ).chain(poly.interiors().iter()))
        .flatten()
        .map(|line_string| {
            line_string.0.iter().circular_tuple_windows::<(&Coordinate<f64>,&Coordinate<f64>)>()

        })
        .flatten()
        .filter(|(&s,&f)|{
            //test the midpoint if it supported
            let mid_point = (s+f )/2.0;
            let supported = archor_area.coordinate_position(&mid_point) != CoordPos::Outside;
            //if midpoint is in the fill area, then it is supported
            !supported
        }).collect();

    println!("unsupported lines {}", unsuported_lines.len());
    unsuported_lines.iter()
        .map(|(line_start,line_end)|
        {
            let projection_sum =0.0;
            let x_diff = line_end.x - line_start.x;
            let y_diff = line_end.y - line_start.y;

            let per_vec = (y_diff,-x_diff);
            let per_vec_len = (((x_diff)*(x_diff))+((y_diff)*(y_diff))).sqrt();

            let projection_sum: f64 = if(per_vec_len !=0.0) {
                unsuported_lines.iter()
                    .map(|(inner_start, inner_end)| {
                        let x_diff = inner_end.x - inner_start.x;
                        let y_diff = inner_end.y - inner_start.y;

                        //println!("vec ({},{})", x_diff, y_diff);

                        let inner_vec = (x_diff, y_diff);

                        let dot = (inner_vec.0 * per_vec.0) + (inner_vec.1 * per_vec.1);

                        (dot / per_vec_len).abs()
                    })
                    .sum()
            }
            else{
                1000000000000000.0
            };
            println!("sum {}", projection_sum);
            (per_vec,projection_sum)
        }).min_by(|(_,l_sum),(_,r_sum) |{

            l_sum.partial_cmp(r_sum).unwrap()
        })
        .map(|((x,y),_)| {
            -90.0 - (y).atan2(x).to_degrees()
        }).unwrap_or(0.0)
}
/*
struct MonotoneSection{
    lower_chain: Vec<Coordinate<f64>>,
    upper_chain: Vec<Coordinate<f64>>,
}

fn get_monotone_sections(poly: &Polygon<f64>) -> Vec<MonotoneSection>{

    let mono_points = poly.exterior().0.iter()
        .circular_tuple_windows::<(Coordinate<f64>,Coordinate<f64>,Coordinate<f64>)>()
        .map(|(p1,p2,p3)|
            {
                if p1.y < p2.y && p3.y < p2.y{
                    //split
                    let (line1, line2) = if p1.x < p3.x{
                        (MonotoneLine{p1: p2, p2: p1},MonotoneLine{p1: p2, p2: p3})
                    }
                    else{
                        (MonotoneLine{p1: p2, p2: p3},MonotoneLine{p1: p2, p2: p1})
                    };
                    MonotonePoint{line1,line2,point_type: PointType::Split}
                }
                else if p1.y > p2.y && p3.y < p2.y{
                    //merge
                    let (line1, line2) = if p1.x < p3.x{
                        (MonotoneLine{p1: p1, p2: p2},MonotoneLine{p1: p3, p2: p2})
                    }
                    else{
                        (MonotoneLine{p1: p3, p2: p2},MonotoneLine{p1: p1, p2: p2})
                    };
                    MonotonePoint{line1,line2,point_type: PointType::Merge}
                }
                else{
                    if p1.y < p3.y{
                        MonotonePoint{line1:MonotoneLine{p1: p3, p2: p2},line2: MonotoneLine{p1: p2, p2: p1},point_type: PointType::Normal}
                    }
                    else{
                        MonotonePoint{line1:MonotoneLine{p1: p1, p2: p2},line2: MonotoneLine{p1: p2, p2: p3},point_type: PointType::Normal}
                    };
                }
            }
        ).collect::<BinaryHeap<MonotonePoint>>();


    vec![]
}

struct MonotonePoint{
    line1: MonotoneLine,
    line2: MonotoneLine,
    point_type: PointType
}

impl PartialOrd for MonotonePoint{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.line1.p2
    }
}


struct MonotoneLine{
    p1: Coordinate<f64>,
    p2: Coordinate<f64>
}

enum PointType{
    Normal,
    Merge,
    Split,
}*/
