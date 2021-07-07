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
use itertools::Itertools;


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
    let (mut vertices,triangles)  =loader.load(matches.value_of("INPUT").unwrap()).unwrap();


    let transform = if let Some(transform_str) = matches.value_of("MANUALTRANFORM") {
        serde_json::from_str(transform_str).unwrap()
    }
    else{
        let (min_x,max_x,min_y,max_y,min_z) = vertices.iter().fold((f64::INFINITY,f64::NEG_INFINITY,f64::INFINITY,f64::NEG_INFINITY,f64::INFINITY), |a,b| (a.0.min(b.x),a.1.max(b.x),a.2.min(b.y),a.3.max(b.y),a.4.min(b.z), ));
        Transform::new_translation_transform( (settings.print_x +max_x+min_x) /2.,(settings.print_y+ max_y+min_y) /2.,-min_z)
    };




    let trans_str = serde_json::to_string(&transform).unwrap();

    println!("transform {}",trans_str);

    for vert in vertices.iter_mut(){
        *vert = &transform * *vert;
    }

    println!("Creating Tower");

    //println!("Here");

    let tower = TriangleTower::from_triangles_and_vertices(&triangles,vertices);

    let mut tower_iter = TriangleTowerIterator::new(&tower);
    //tower_iter.advance_to_height(0.5);

    //for vert in &tower_iter.get_points()[0] {
    //    println!("{},{}",vert.x,vert.y);
    //}

    println!("Slicing");

    let mut moves = vec![];
    let mut layer = settings.first_layer_height;
    let mut more_lines = true;



    let mut  slices = vec![];

    while more_lines {
        tower_iter.advance_to_height(layer );

        //println!("layer {}",layer);

        let layer_loops = tower_iter.get_points();

        if layer_loops.is_empty(){
            more_lines = false;
        }
        else {
            let slice = Slice::from_multiple_point_loop(layer_loops.iter().map(|verts| verts.into_iter().map(|v| Coordinate { x: v.x, y: v.y }).collect::<Vec<Coordinate<f64>>>()).collect());

            slices.push((layer,slice));
        };

        layer += settings.layer_height;
        //println!("laye2 {}",layer)

    }
    println!("Generating Moves");

    let mut layer_count = 0;

    let slice_count = slices.len();

    for (layer,slice) in slices.iter_mut(){
        moves.push(Command::LayerChange {z: *layer});

        println!("layer {} {}", layer_count ,layer_count < 3 || layer_count+ 3 +1>slice_count );


        slice.slice_into_commands(&settings,&mut moves, layer_count < 3 || layer_count+ 3 +1>slice_count );






        layer_count +=1;

    }



    if let Some(file_path ) = matches.value_of("OUTPUT"){
        println!("Optimizing {} Moves", moves.len());
        optimize_commands(&mut moves,&settings);
        println!("Converting {} Moves", moves.len());
        convert(&moves,settings,&mut File::create(file_path).expect("File not Found")).unwrap();
    }
    else{

        println!("Optimizing {} Moves", moves.len());
        let stdout = std::io::stdout();
        optimize_commands(&mut moves,&settings);
        println!("Converting {} Moves", moves.len());
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
            Command::MoveTo { end,..} => {
                writeln!(write,"G1 X{:.5} Y{:.5}",end.x,end.y )?
            },
            Command::MoveAndExtrude {start,end} => {
                let x_diff = end.x-start.x;
                let y_diff = end.y-start.y;
                let length = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();

                let extrude = ((4.0 * settings.layer_height * settings.layer_width) /(std::f64::consts::PI*settings.filament.diameter*settings.filament.diameter)) *length;

                writeln!(write,"G1 X{:.5} Y{:.5} E{:.5}",end.x ,end.y,extrude)?;
            }
            Command::SetState {new_state} => {

                match new_state.Retract{
                    None => {}
                    Some(dir) => {
                        writeln!(write, "G1 E{} F{} ; Retract or unretract",if dir {-1.0} else {1.0} * settings.retract_length, 60.0 * settings.retract_speed)?;
                    }
                }

                if let Some(speed)  = new_state.MovementSpeed{
                    writeln!(write,"G1 F{:.5}",speed * 60.0 )?;
                }
                if let Some(ext_temp)  = new_state.ExtruderTemp{
                     writeln!(write,"M104 S{:.1} ; set extruder temp",ext_temp )?;
                }
                if let Some(bed_temp)  = new_state.BedTemp{
                     writeln!(write,"M140 S{:.1} ; set bed temp",bed_temp )?;
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
                //println!("{}",t);
                //Find the inverse sine of the result (in radians).
                //Double the result of the inverse sine to get the central angle in radians.
                let central = t.asin() *2.0;
                //Once you have the central angle in radians, multiply it by the radius to get the arc length.
                let extrusion_length  = central * radius;

                //println!("{}",extrusion_length);
                let extrude = (4.0 * settings.layer_height * settings.layer_width*extrusion_length) /(std::f64::consts::PI*settings.filament.diameter*settings.filament.diameter);
                writeln!(write,"{} X{:.5} Y{:.5} I{:.5} J{:.5} E{:.5}",if *clockwise { "G2"} else{"G3"},end.x ,end.y,center.x, center.y, extrude)?;


            }
            Command::NoAction =>{
                panic!("Converter reached a No Action Command, Optimization Failure")
            }
        }
    }

     let end = settings.ending_gcode.clone();

    writeln!(write,"{}",end)?;

    Ok(())

}