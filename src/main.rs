use clap::{App, load_yaml};
use simple_logger::SimpleLogger;
use log::{LevelFilter};
use crate::types::*;
use crate::loader::*;

use crate::settings::Settings;
use std::io::Write;
use std::fs::File;
use crate::plotter::Slice;
use crate::optimizer::optimize_commands;
use crate::tower::*;
use geo::Coordinate;


mod loader;
mod types;
mod settings;
mod plotter;
mod optimizer;
mod tower;

fn main() {


    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let settings : Settings = matches.value_of("SETTINGS").map(|str| serde_json::from_str(&std::fs::read_to_string(str).unwrap()).unwrap() ).unwrap_or_default();

        // Gets a value for config if supplied by user, or defaults to "default.conf"
    let config = matches.value_of("config").unwrap_or("default.conf");
    println!("Value for config: {}", config);

    // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
    // required we could have used an 'if let' to conditionally get the value)
    println!("Using input file: {}", matches.value_of("INPUT").unwrap());

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    match matches.occurrences_of("verbose") {
        0 => SimpleLogger::new().with_level(LevelFilter::Error ).init().unwrap(),
        1 => SimpleLogger::new().with_level(LevelFilter::Warn ).init().unwrap(),
        2 => SimpleLogger::new().with_level(LevelFilter::Info ).init().unwrap(),
        3 => SimpleLogger::new().with_level(LevelFilter::Debug ).init().unwrap(),
        4 | _ => SimpleLogger::new().with_level(LevelFilter::Trace ).init().unwrap(),
    }

    println!("Loading Input");

    let loader = STLLoader{};
    let (vertices,triangles)  =loader.load(matches.value_of("INPUT").unwrap()).unwrap();

    println!("Creating Tower");

    //println!("Here");

    let tower = TriangleTower::from_triangles_and_vertices(&triangles,vertices);

    let mut tower_iter = TriangleTowerIterator::new(&tower);
    //tower_iter.advance_to_height(0.5);

    //for vert in &tower_iter.get_points()[0] {
    //    println!("{},{}",vert.x,vert.y);
    //}

    println!("Generating Moves");

    let mut moves = vec![];
    let mut layer = settings.layer_height/2.0;
    let mut more_lines = true;
    while more_lines {
        tower_iter.advance_to_height(layer );

        //println!("layer {}",layer);
        let slices = tower_iter.get_points();

        let slice= Slice::from_multiple_point_loop(slices.iter().map( |verts| verts.into_iter().map(|v| Coordinate { x: v.x,y:v.y}  ).collect::<Vec<Coordinate<f64>>>()).collect());
        slice.slice_into_commands(&settings,&mut moves);

        if slices.is_empty(){
            more_lines = false;
        }
        else {
            moves.push(Command::LayerChange {z: layer + settings.layer_height});
            layer += settings.layer_height;
        }

    }

    if let Some(file_path ) = matches.value_of("OUTPUT"){
        println!("Optimizing");
        optimize_commands(&mut moves);
        println!("Converting");
        convert(&moves,settings,&mut File::create(file_path).expect("File not Found")).unwrap();
    }
    else{
        println!("Optimizing");
        let stdout = std::io::stdout();
        optimize_commands(&mut moves);
        println!("Converting");
        convert(&moves,settings,&mut stdout.lock()).unwrap();

    };






}

/*
fn triangle_z_intersection(z: f32, triangle : Triangle) -> Option<(Vertex,Vertex)>
{
    let v0 = triangle.vertices[0];
    let v1 = triangle.vertices[1];
    let v2 = triangle.vertices[2];

    let v0_1_intersection = {
        if (v0[2] > z) != (v1[2] > z) {
            // one above and one below
           line_z_intersection(z,v0,v1)
        }
        else{
            None
        }
    };
    let v1_2_intersection = {
        if (v1[2] > z) != (v2[2] > z) {
            // one above and one below
           line_z_intersection(z,v1,v2)
        }
        else{
            None
        }
    };
    let v2_0_intersection = {
        if (v2[2] > z) != (v0[2] > z) {
            // one above and one below
           line_z_intersection(z,v2,v0)
        }
        else{
            None
        }
    };
    if let Some(r1) = v0_1_intersection{

        if let Some(r2) = v1_2_intersection {
            Some((r1,r2))
        }
        else if let Some(r2) = v2_0_intersection {
            Some((r1,r2))
        }
        else
        {
            None
        }
    }
    else if let Some(r1) = v1_2_intersection{

        if let Some(r2) = v2_0_intersection {
            Some((r1,r2))
        }
        else
        {
            None
        }
    }
    else{
        None

    }
}*/




/*fn deindex_triangle(vertices: &Vec<Vertex> , tri : &IndexedTriangle) -> Triangle{
    Triangle{normal: tri.normal,vertices: [vertices[tri.vertices[0]],vertices[tri.vertices[1]],vertices[tri.vertices[2]]]}
}*/



fn convert( cmds: &Vec<Command>, settings: Settings, write:&mut impl Write) ->  Result<(),Box<dyn std::error::Error>>{

    let mut start = settings.starting_gcode.clone();

    start = start.replace("[First Layer Extruder Temp]", &format!("{:.1}",settings.filament.extruder_temp));
    start = start.replace("[First Layer Bed Temp]", &format!("{:.1}",settings.filament.bed_temp));

    writeln!(write,"{}",start)?;

    for cmd in cmds{
        match cmd {
            Command::MoveTo { end} => {
                writeln!(write,"G1 X{:.5} Y{:.5}",end.x+100.0,end.y+100.0 )?
            },
            Command::MoveAndExtrude {start,end} => {
                let x_diff = end.x-start.x;
                let y_diff = end.y-start.y;
                let length = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();

                let extrude = (4.0 * settings.layer_height * settings.layer_width*length) /(std::f64::consts::PI*settings.filament.diameter*settings.filament.diameter);
                writeln!(write,"G1 X{:.5} Y{:.5} E{:.5}",end.x +100.0,end.y+100.,extrude)?;
            }
            Command::SetState {new_state} => {
                if let Some(speed)  = new_state.MovementSpeed{
                    writeln!(write,"G1 F{:.5}",speed * 60.0 )?;
                }
                if let Some(ext_temp)  = new_state.ExtruderTemp{
                     writeln!(write,"M104 S{:.1} ; set extruder temp",ext_temp )?;
                }
            }
            Command::LayerChange {z} => {
                writeln!(write,"G1 Z{:.5}",z )?;
            }
            Command::Delay {msec} =>
            {
                writeln!(write,"G4 P{:.5}",msec )?;
            }
            Command::Arc { start,end,center,clockwise} => {
                let x_diff = end.x-start.x;
                let y_diff = end.y-start.y;
                let cord_length = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();
                let x_diff_r = end.x-center.x;
                let y_diff_r = end.y-center.y;
                let radius = ((x_diff_r * x_diff_r) + (y_diff_r * y_diff_r)).sqrt();

                //Divide the chord length by double the radius.
                let t = cord_length / (2.0*radius);
                //Find the inverse sine of the result (in radians).
                //Double the result of the inverse sine to get the central angle in radians.
                let central = t.asin() *2.0;
                //Once you have the central angle in radians, multiply it by the radius to get the arc length.
                let extrusion_length  = central * radius;

                let extrude = (4.0 * settings.layer_height * settings.layer_width*extrusion_length) /(std::f64::consts::PI*settings.filament.diameter*settings.filament.diameter);
                writeln!(write,"{} X{:.5} Y{:.5} E{:.5}",if *clockwise { "G2"} else{"G3"},end.x +100.0,end.y+100.,extrude)?;


            }
        }
    }

     let end = settings.ending_gcode.clone();

    writeln!(write,"{}",end)?;

    Ok(())

}