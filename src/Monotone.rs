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

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::ops::Index;
use geo::{Coordinate, Polygon};
use geo::prelude::*;
use itertools::Itertools;

#[derive( Debug)]
pub struct MonotoneSection{
    pub left_chain: Vec<Coordinate<f64>>,
    pub right_chain: Vec<Coordinate<f64>>,
}


#[derive(Debug, PartialEq)]
struct MonotonePoint{
    pos: Coordinate<f64>,
    next: Coordinate<f64>,
    prev: Coordinate<f64>,
    point_type: PointType
}


impl PartialOrd for MonotonePoint{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.pos.y.partial_cmp(&other.pos.y).map(|cmp| cmp.then(self.pos.x.partial_cmp(&other.pos.x).unwrap()))
    }
}

impl Eq for MonotonePoint {}

impl Ord for MonotonePoint{
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
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

pub fn get_monotone_sections(poly: &Polygon<f64>) -> Vec<MonotoneSection>{

    println!("size {:?}", poly.exterior().0);

    let mut mono_points = std::iter::once(poly.exterior()).chain(poly.interiors().iter().rev())
        .map(|line_string|{
            line_string.0
                .iter()
                .take(poly.exterior().0.len() -1)
                .circular_tuple_windows::<(&Coordinate<f64>,&Coordinate<f64>,&Coordinate<f64>)>()
                        .map(|(&next, &point, &prev)| {

                    let point_type = if isabove(&point,&prev) && isabove(&point,&next) {
                        if orientation(&prev,&point,&next) == Orientation::Left{
                            println!("add split");
                            PointType::Split
                        }
                        else{
                            println!("add start");
                            PointType::Start
                        }
                    }
                    else if !isabove(&point,&prev) && !isabove(&point,&next){
                        if orientation(&prev,&point,&next) == Orientation::Left{
                            println!("add merge");
                            PointType::Merge
                        }
                        else{
                            println!("add end");
                            PointType::End
                        }
                    }
                    else if isabove(&point,&prev) && !isabove(&point,&next){
                        println!("add left");
                        PointType::Left
                    }
                    else{
                        println!("add right");
                        PointType::Right
                    };

                    MonotonePoint{
                        pos: point,
                        next,
                        prev,
                        point_type
                    }

                })
        })
        .flatten()
        .collect::<BinaryHeap<MonotonePoint>>();

    let mut sweep_line_storage : Vec<MonotoneSection>=  vec![];
    let mut completed_sections : Vec<MonotoneSection>=  vec![];

    while let Some(point) = mono_points.pop() {

         println!("Type: {:?}", point.point_type);
         //println!("sweep: {:?}", sweep_line_storage);
        match point.point_type {
            PointType::Start => {

                let new_section = MonotoneSection{
                    left_chain: vec![point.pos,point.prev],
                    right_chain: vec![point.pos,point.next],
                };


                let index= sweep_line_storage.iter().position(|section| {
                    let right_top = section.right_chain.get(section.right_chain.len() - 2).unwrap();
                    let right_bot = section.right_chain.last().unwrap();

                    let right_x = point_lerp(right_top, right_bot, point.pos.y).x;
                    point.pos.x < right_x
                }).unwrap_or(sweep_line_storage.len());
                sweep_line_storage.insert(index, new_section);

                println!("Start  {:?}" , point.pos);
                println!("Start add {:?} {:?}" , point.prev, point.next);

            }
            PointType::End => {
                let index = sweep_line_storage.iter().position(|section| *section.left_chain.last().unwrap() == point.pos).expect(format!("End point must be in the storage {:?} |||| {:?}", point, sweep_line_storage).as_str());

                let removed_section = sweep_line_storage.remove(index);

                completed_sections.push(removed_section);

            }
            PointType::Left =>{
                let index = sweep_line_storage.iter().position(|section| *section.left_chain.last().unwrap() == point.pos).expect(format!("left error {:?} {:?}", point, sweep_line_storage).as_str());

                sweep_line_storage[index].left_chain.push(point.prev);

                println!("Left add {:?}" , point.prev);
            }
            PointType::Right =>{
                let index = sweep_line_storage.iter().position(|section| *section.right_chain.last().unwrap() == point.pos).expect(format!("right error {:?} {:?}", point, sweep_line_storage).as_str());

                sweep_line_storage[index].right_chain.push(point.next);

                println!("Right add {:?}" , point.next);
            }
            PointType::Merge => {
                let index = sweep_line_storage.iter().position(|section| *section.right_chain.last().unwrap() == point.pos).expect(format!("Merge point must be in the storage as the end of a chain{:?} |||| {:?}", point, sweep_line_storage).as_str());
                println!("Merge {:?}" , &point.pos);
                let mut right_section = sweep_line_storage.remove(index+1);
                let left_section = &mut sweep_line_storage[index];



                assert_eq!(*left_section.right_chain.last().unwrap() , *right_section.left_chain.last().unwrap());

                //The new point generated on the right most edge
                let break_point_low = right_section.right_chain.pop().unwrap();
                let break_point_high = right_section.right_chain.last().unwrap();

                let break_point = point_lerp(break_point_high,&break_point_low,point.pos.y);

                right_section.right_chain.push(break_point);

                completed_sections.push(right_section);

                left_section.right_chain.push(break_point);
                left_section.right_chain.push(break_point_low);

                println!("Merge break {:?}" , break_point);
                println!("Merge end {:?}" , break_point_low);


            }

            PointType::Split => {
                //find the section that will be split up
                let index = sweep_line_storage.iter().position(|section| {
                    let left_top = section.left_chain.get(section.left_chain.len()-2).unwrap();
                    let left_bot = section.left_chain.last().unwrap();
                    let right_top = section.right_chain.get(section.right_chain.len()-2).unwrap();
                    let right_bot = section.right_chain.last().unwrap();

                    let left_x = point_lerp(left_top,left_bot,point.pos.y).x;
                    let right_x = point_lerp(right_top,right_bot,point.pos.y).x;


                    point.pos.x > left_x && point.pos.x < right_x
                }).expect(format!("split error {:?} {:?}", point, sweep_line_storage).as_str());

                //will become new left section
                let old_section = sweep_line_storage.get_mut(index).unwrap();

                let break_point_low = old_section.right_chain.pop().unwrap();
                let break_point_high = old_section.right_chain.last().unwrap();

                let break_point = point_lerp(break_point_high,&break_point_low,point.pos.y);

                old_section.right_chain.push(break_point);
                old_section.right_chain.push(point.pos);
                old_section.right_chain.push(point.next);



                let new_right_section = MonotoneSection{
                    left_chain: vec![point.pos, point.prev],
                    right_chain: vec![break_point, break_point_low],
                };

                sweep_line_storage.insert(index+1, new_right_section);

            }
        }
    }

    println!("Monotone sections {}",completed_sections.len());

    completed_sections
}


fn isabove(a:& Coordinate<f64>,b: &Coordinate<f64>) -> bool
{
    a.y.partial_cmp(&b.y).map(|cmp| cmp.then(a.x.partial_cmp(&b.x).unwrap())).unwrap() == Ordering::Greater
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

#[inline]
fn point_lerp(a: &Coordinate<f64>, b: &Coordinate<f64>, y: f64) -> Coordinate<f64>{
    Coordinate{
        x : lerp(a.x,b.x, ((y-a.y)/(b.y-a.y))),
        y ,
    }
}

#[inline]
fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + f * (b - a)
}