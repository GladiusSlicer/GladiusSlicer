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
*/
/*
use std::collections::{HashMap, HashSet, VecDeque};
use std::iter::Map;
use geo::{Coordinate, point};
use itertools::Itertools;
use serde::__private::de::ContentRefDeserializer;
use crate::TriangleTowerIterator;

enum PointType{
    Start,
    End,
    Merge,
    Split,
    Left,
    Right
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Orientation{
    Linear,
    Left,
    Right
}

struct Point{
    x: f64,
    y: f64,
    point_type: PointType
}


fn splitMonotone(points :  Vec<Coordinate<f64>>) {
    let mut monotone_points: Vec<Point> = points
        .iter()
        .circular_tuple_windows::<(&Coordinate<f64>, &Coordinate<f64>, &Coordinate<f64>)>()
        .map(|(prev, point, next)| {


            let point_type = if isabove(point,prev) && isabove(point,next) {
                if orientation(prev,point,next) == Orientation::Left{
                    PointType::Start
                }
                else{
                    PointType::Split
                }
            }
            else if isabove(prev,point) && isabove(next,point){
                if orientation(prev,point,next) == Orientation::Left{
                    PointType::End
                }
                else{
                    PointType::Merge
                }
            }
            else if isabove(point,prev) && isabove(point,next){
                PointType::Left
            }
            else{
                PointType::Right
            };

            Point{
                x: point.x,
                y: point.y,
                point_type
            }

        })
        .collect();

    let mut edl : Vec<(Coordinate<f64>,Coordinate<f64>,usize)> = vec![];
    let mut rep : HashMap<Coordinate<f64>, usize> = HashMap::new();

    let mut id =0;

    for ( index,( cur, next)) in monotone_points
        .iter()
        .map(|p|{
            Coordinate{
                x: p.x,
                y: p.y
            }
        })
        .circular_tuple_windows::<(Coordinate<f64>, Coordinate<f64>)>()
        .enumerate()
    {
        rep.insert(cur,id);

        if next.y <= cur.y{
            edl.push((cur,next,index));
            id+=1;
        }
    }

    let mut helpers : Vec<Option<usize>> = Vec::with_capacity(monotone_points.len());

    for _ in (0..monotone_points.len()){
        helpers.push(None);
    }

    let mut queue :  Vec<(usize,Point)> = monotone_points.iter()
        .enumerate()
        .collect();

    let mut stat = HashSet::new();

    let mut diagonals = vec![];

    queue.sort_by(|a,b|(-a.1.y).partial_cmp(&-b.1.y).unwrap().then(a.1.x.partial_cmp(&b.1.x).unwrap()));

    for (id , point) in queue{
        match point.point_type {
            PointType::Start => {
                handleStart(point,id,&mut helpers,&mut stat,& rep);
            }
            PointType::Split =>{
                handleSplit(point,id,&mut helpers,&mut edl,&mut stat,&rep, &mut diagonals)
            }
            PointType::End =>{

            }
            PointType::Merge =>{

            }
            PointType::Left =>{

            }
            PointType::Right =>{

            }
        }
    }
}

fn isabove(a:& Coordinate<f64>,b: &Coordinate<f64>) -> bool
{
    if a.y!=b.y {
        a.y>b.y
    }
    else{
        a.x<b.x
    }
}

fn orientation(p : &Coordinate<f64>,q : &Coordinate<f64>,r : &Coordinate<f64>) -> Orientation
{
    let val=(q.x-p.x)*(r.y-p.y)-(q.y-p.y)*(r.x-p.x);
    if val==0.0 {
        Orientation::Linear
    }
    else if val>0.0 {

        Orientation::Left
    }
    else {
        Orientation::Right
    }
}

fn handleStart(p:Point,id: usize,helpers:&mut Vec<Option<usize>>,s : &mut HashSet<usize> ,mm : &HashMap<Coordinate<f64>,usize> )
{
    let pp= Coordinate{x:p.x,y:p.y};
    let eid=mm[&pp];
    helpers[eid]=Some(id);
    s.insert(eid);
}

fn handleSplit(p:Point,id: usize,helpers:&mut Vec<Option<usize>>,edl : &mut Vec<(Coordinate<f64>,Coordinate<f64>,usize)> ,s : &mut HashSet<usize> ,mm : &HashMap<Coordinate<f64>,usize> ,&mut diagonals: &mut Vec<(usize,usize)>)
{
    let pp= Coordinate{x:p.x,y:p.y};
    let lid = getleft(p,edl,s).unwrap();

    diagonals.push((id,lid));
    let eid=mm[&pp];
    helpers[lid]=Some(id);
    s.insert(mm.get(p).unwrap());
}

fn getleft(p:Point,edl : &mut Vec<(Coordinate<f64>,Coordinate<f64>,usize)>,s : &mut HashSet<usize>) -> Option<usize>
{
    let mut ret = None;
    let mut md=1e7;
    for (index,edli) in edl.iter().enumerate()
    {
        if(orientation(&edli.1,&edli.0,&Coordinate{x:p.x,y:p.y}) != Orientation::Left && edli.1.y<=p.y && p.y<=edli.0.y &&(p.x!=edli.0.x&&p.y!=edli.0.y))
        {
            let dx=(edli.0.x-p.x).abs();
            if(dx<md)
            {
                md=dx;
                ret=Some(index);
            }
        }
    }
    return ret;
}
fn getright(p:Point,edl : &mut Vec<(Coordinate<f64>,Coordinate<f64>,usize)>,s : &mut HashSet<usize>) -> usize
{
    let mut ret = None;
    let mut md=1e7;
    for (index,edli) in edl.iter().enumerate()
    {
        if(orientation(&edli.1,&edli.0,&Coordinate{x:p.x,y:p.y})!= Orientation::Right && edli.1.y<=p.y && p.y<=edli.0.y &&(p.x!=edli.0.x&&p.y!=edli.0.y))
        {
            let dx=(edli.0.x-p.x).abs();
            if(dx<md)
            {
                md=dx;
                ret=Some(index);
            }
        }
    }
    return ret;
}

*/

