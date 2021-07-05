use geo::*;
use geo_clipper::*;
use geo::prelude::*;
use crate::types::{Command, StateChange, Move, MoveType, MoveChain};
use crate::settings::Settings;
use itertools::{Itertools, chain};
use std::iter::FromIterator;
use std::collections::VecDeque;
use ordered_float::OrderedFloat;

pub struct Slice{
    MainPolygon: MultiPolygon<f64>,
    ouline_area: Option<MultiPolygon<f64>>,
    solid_infill: Option<MultiPolygon<f64>>,
    normal_infill: Option<MultiPolygon<f64>>,
}

impl Slice{
    pub fn from_single_point_loop< I>( line: I) -> Self where I: Iterator<Item = (f64,f64)> {
        let polygon = Polygon::new(
            LineString::from_iter(line) ,
            vec![],
        );

        Slice{MainPolygon: MultiPolygon(vec![polygon]),ouline_area: None,solid_infill:None,normal_infill: None}
    }

     pub fn from_multiple_point_loop( lines: MultiLineString<f64>)  -> Self{

         let mut polygons : Vec<Polygon<f64>> = vec![];

         for line in lines.iter(){

             let new_polygon = Polygon::new(line.clone(), vec![]);

             'outer : for polygon in polygons.iter_mut(){
                 if polygon.contains(&new_polygon){
                     polygon.interiors_push(line.clone());
                     break 'outer;
                 }
             }

             polygons.push(new_polygon);
         }
        let multi_polygon :MultiPolygon<f64> = MultiPolygon(polygons);

        Slice{MainPolygon: multi_polygon,ouline_area: None,solid_infill:None,normal_infill: None}
    }

    pub fn slice_into_commands(&self,settings:&Settings, commands: &mut Vec<Command>, solid: bool) {

        let mut current_mulipoly = self.MainPolygon.clone();


        let mut chains = vec![];

        for _ in 0..3{
            let (m,mut new_chains) =  inset_polygon(&current_mulipoly,settings);
            current_mulipoly = m;
            chains.append(&mut new_chains);
        }

        for poly in current_mulipoly
        {
            if solid{
                let new_moves = solid_fill_polygon(&poly,settings);


                if let Some(chain) = new_moves{
                    chains.push(chain);
                }

            }
            else{
                let new_moves = partial_fill_polygon(&poly,settings,settings.infill_percentage);

                if let Some(chain) = new_moves{
                    chains.push(chain);
                }
            }

        }

        let mut ordered_chains = vec![chains.swap_remove(0)];

        while !chains.is_empty(){
            let index = chains.iter().position_min_by_key(|a|OrderedFloat(ordered_chains.last().unwrap().moves.last().unwrap().end.euclidean_distance(&a.start_point))).unwrap();
            let closest_chain = chains.remove(index);
            ordered_chains.push(closest_chain);
        }

        let mut full_moves = vec![];
        let starting_point =  ordered_chains[0].start_point;
        for mut chain in ordered_chains{
            full_moves.push(Move{end: chain.start_point ,move_type: MoveType::Travel });
            full_moves.append(&mut chain.moves)
        }

        commands.append(&mut MoveChain{moves:full_moves,start_point: starting_point}.create_commands(settings));

    }
}


fn inset_polygon( poly: &MultiPolygon<f64>, settings : &Settings) -> (MultiPolygon<f64>,Vec<MoveChain>){

    let mut move_chains =  vec![];
    let inset_poly = poly.offset(-settings.layer_width/2.0,JoinType::Miter(10.0),EndType::ClosedPolygon,1000000.0);

    for polygon in inset_poly.0.iter()
    {
        let mut moves = vec![];


        for (&start,&end) in polygon.exterior().0.iter().circular_tuple_windows::<(_,_)>(){
            moves.push(Move{end: end,move_type: MoveType::Outer_Perimeter});
        }

        move_chains.push(MoveChain{start_point:polygon.exterior()[0], moves});

        for interior in polygon.interiors() {
            let mut moves = vec![];
            for (&start, &end) in interior.0.iter().circular_tuple_windows::<(_, _)>() {
                moves.push(Move{end: end,move_type: MoveType::Outer_Perimeter});
            }
            move_chains.push(MoveChain{start_point:interior.0[0], moves});
        }

    }

    (inset_poly.offset(-settings.layer_width/2.0,JoinType::Miter(10.0),EndType::ClosedPolygon,1000000.0),move_chains)
}

fn solid_fill_polygon( poly: &Polygon<f64>, settings : &Settings) -> Option<MoveChain> {
    let mut moves =  vec![];

    let mut lines : Vec<(Coordinate<f64>,Coordinate<f64>)> = poly.exterior().0.iter().map(|c| *c).circular_tuple_windows::<(_, _)>().collect();

    for interior in poly.interiors(){
        let mut new_lines = interior.0.iter().map(|c| *c).circular_tuple_windows::<(_, _)>().collect();
        lines.append(&mut new_lines);
    }

    for line in lines.iter_mut(){
        *line = if line.0.y < line.1.y {
            *line
        }
        else{
            (line.1,line.0)
        };
    };

    lines.sort_by(|a,b| b.0.y.partial_cmp(&a.0.y).unwrap());

    let mut current_y = lines[lines.len() -1].0.y + settings.layer_width/2.0;

    let mut current_lines = Vec::new();

    let mut orient = false;

    let mut start_point = None;

    let mut line_change = false;

    while !lines.is_empty(){
        line_change = false;
        while !lines.is_empty() && lines[lines.len() -1].0.y < current_y{

            current_lines.push(lines.pop().unwrap());
            line_change = true;
        }


        if lines.is_empty(){
            break;
        }

        current_lines.retain(|(s,e)| e.y > current_y );



        //current_lines.sort_by(|a,b| b.0.x.partial_cmp(&x.0.y).unwrap().then(b.1.x.partial_cmp(&a.1.x).unwrap()) )

        let mut points = current_lines.iter().map(|(start,end)| {
            let x = ((current_y- start.y) * ((end.x - start.x)/(end.y - start.y))) + start.x;
            x
        }).collect::<Vec<_>>();

        points.sort_by(|a,b| a.partial_cmp(b).unwrap());

        start_point = start_point.or(Some(Coordinate{x: points[0], y: current_y}));

        moves.push(Move{ end: Coordinate{x: points[0], y: current_y},move_type: MoveType::Travel});

        if orient {
            for (start, end) in points.iter().tuples::<(_, _)>() {
                if !line_change{
                    moves.push(Move{ end: Coordinate { x: *start, y: current_y },move_type: MoveType::SolidInfill} );
                } else{
                    moves.push(Move{ end: Coordinate { x: *start, y: current_y },move_type: MoveType::Travel} );
                }
                moves.push(Move{ end: Coordinate { x: *end, y: current_y }  ,move_type: MoveType::SolidInfill} );
            }
        }
        else{
            for (start, end) in points.iter().rev().tuples::<(_, _)>() {
                if !line_change{
                    moves.push(Move{ end: Coordinate { x: *start, y: current_y },move_type: MoveType::SolidInfill} );
                } else{
                    moves.push(Move{ end: Coordinate { x: *start, y: current_y },move_type: MoveType::Travel} );
                }
                moves.push(Move{ end: Coordinate { x: *end, y: current_y }  ,move_type: MoveType::SolidInfill} );
            }
        }

        orient = !orient;
        current_y += settings.layer_width;

    }


    start_point.map(|start_point|MoveChain{moves,start_point })

}

fn partial_fill_polygon( poly: &Polygon<f64>, settings : &Settings, fill_ratio: f64) -> Option<MoveChain> {
    let mut moves =  vec![];

    let mut lines : Vec<(Coordinate<f64>,Coordinate<f64>)> = poly.exterior().0.iter().map(|c| *c).circular_tuple_windows::<(_, _)>().collect();

    for interior in poly.interiors(){
        let mut new_lines = interior.0.iter().map(|c| *c).circular_tuple_windows::<(_, _)>().collect();
        lines.append(&mut new_lines);
    }

    for line in lines.iter_mut(){
        *line = if line.0.y < line.1.y {
            *line
        }
        else{
            (line.1,line.0)
        };
    };

    lines.sort_by(|a,b| b.0.y.partial_cmp(&a.0.y).unwrap());

    let mut current_y = lines[lines.len() -1].0.y + settings.layer_width/2.0;

    let mut current_lines = Vec::new();

    let mut orient = false;

    let mut start_point = None;

    let mut line_change = false;

    let distance = settings.layer_width / fill_ratio;

    while !lines.is_empty(){
        line_change = false;
        while !lines.is_empty() && lines[lines.len() -1].0.y < current_y{

            current_lines.push(lines.pop().unwrap());
            line_change = true;
        }


        if lines.is_empty(){
            break;
        }

        current_lines.retain(|(s,e)| e.y > current_y );



        //current_lines.sort_by(|a,b| b.0.x.partial_cmp(&x.0.y).unwrap().then(b.1.x.partial_cmp(&a.1.x).unwrap()) )

        let mut points = current_lines.iter().map(|(start,end)| {
            let x = ((current_y- start.y) * ((end.x - start.x)/(end.y - start.y))) + start.x;
            x
        }).collect::<Vec<_>>();

        points.sort_by(|a,b| a.partial_cmp(b).unwrap());

        start_point = start_point.or(Some(Coordinate{x: points[0], y: current_y}));

        moves.push(Move{ end: Coordinate{x: points[0], y: current_y},move_type: MoveType::Travel});

        if orient {
            for (start, end) in points.iter().tuples::<(_, _)>() {
                if !line_change{
                    moves.push(Move{ end: Coordinate { x: *start, y: current_y },move_type: MoveType::SolidInfill} );
                } else{
                    moves.push(Move{ end: Coordinate { x: *start, y: current_y },move_type: MoveType::Travel} );
                }
                moves.push(Move{ end: Coordinate { x: *end, y: current_y }  ,move_type: MoveType::SolidInfill} );
            }
        }
        else{
            for (start, end) in points.iter().rev().tuples::<(_, _)>() {
                if !line_change{
                    moves.push(Move{ end: Coordinate { x: *start, y: current_y },move_type: MoveType::SolidInfill} );
                } else{
                    moves.push(Move{ end: Coordinate { x: *start, y: current_y },move_type: MoveType::Travel} );
                }
                moves.push(Move{ end: Coordinate { x: *end, y: current_y }  ,move_type: MoveType::SolidInfill} );
            }
        }

        orient = !orient;
        current_y += distance;

    }


    start_point.map(|start_point|MoveChain{moves,start_point })

}





