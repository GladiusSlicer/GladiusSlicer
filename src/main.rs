use crate::loader::*;
use crate::types::*;
use clap::{load_yaml, App};
use log::LevelFilter;
use simple_logger::SimpleLogger;

use crate::optimizer::optimize_commands;
use crate::plotter::Slice;
use crate::settings::Settings;
use crate::tower::*;
use geo::Coordinate;
use geo_clipper::*;
use std::fs::File;
use std::io::{BufWriter, Write};

use geo::prelude::ConvexHull;

use std::ffi::OsStr;
use std::path::Path;

use itertools::{Itertools};
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use std::collections::HashMap;

mod Monotone;
mod loader;
mod optimizer;
mod plotter;
mod settings;
mod tower;
mod types;

fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let settings: Settings = matches
        .value_of("SETTINGS")
        .map(|str| serde_json::from_str(&std::fs::read_to_string(str).unwrap()).unwrap())
        .unwrap_or_default();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let config = matches.value_of("config").unwrap_or("default.conf");
    println!("Value for config: {}", config);

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    match matches.occurrences_of("verbose") {
        0 => SimpleLogger::new()
            .with_level(LevelFilter::Error)
            .init()
            .unwrap(),
        1 => SimpleLogger::new()
            .with_level(LevelFilter::Warn)
            .init()
            .unwrap(),
        2 => SimpleLogger::new()
            .with_level(LevelFilter::Info)
            .init()
            .unwrap(),
        3 => SimpleLogger::new()
            .with_level(LevelFilter::Debug)
            .init()
            .unwrap(),
        _ => SimpleLogger::new()
            .with_level(LevelFilter::Trace)
            .init()
            .unwrap(),
    }

    println!("Loading Input");

    let converted_inputs: Vec<(Vec<Vertex>, Vec<IndexedTriangle>)> = matches
        .values_of("INPUT")
        .unwrap()
        .map(|value| {
            let obj: InputObject = serde_json::from_str(value).unwrap();
            obj
        })
        .par_bridge()
        .map(|object| {
            let model_path = Path::new(object.get_model_path());

            // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
            // required we could have used an 'if let' to conditionally get the value)
            println!("Using input file: {:?}", model_path);

            let extension = model_path
                .extension()
                .and_then(OsStr::to_str)
                .expect("File Parse Issue");

            let loader: &dyn Loader = match extension {
                "stl" => &STLLoader {},
                "3MF" | "3mf" => &ThreeMFLoader {},
                _ => panic!("File Format {} not supported", extension),
            };

            let (mut vertices, triangles) = loader.load(model_path.to_str().unwrap()).unwrap();

            let transform = match object {
                InputObject::Raw(_, transform) => transform,
                InputObject::Auto(_) => {
                    let (min_x, max_x, min_y, max_y, min_z) = vertices.iter().fold(
                        (
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                            f64::INFINITY,
                        ),
                        |a, b| {
                            (
                                a.0.min(b.x),
                                a.1.max(b.x),
                                a.2.min(b.y),
                                a.3.max(b.y),
                                a.4.min(b.z),
                            )
                        },
                    );
                    Transform::new_translation_transform(
                        (settings.print_x - (max_x + min_x)) / 2.,
                        (settings.print_y - (max_y + min_y)) / 2.,
                        -min_z,
                    )
                }
                InputObject::AutoTranslate(_, x, y) => {
                    let (min_x, max_x, min_y, max_y, min_z) = vertices.iter().fold(
                        (
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                            f64::INFINITY,
                        ),
                        |a, b| {
                            (
                                a.0.min(b.x),
                                a.1.max(b.x),
                                a.2.min(b.y),
                                a.3.max(b.y),
                                a.4.min(b.z),
                            )
                        },
                    );
                    Transform::new_translation_transform(
                        (x + settings.print_x - (max_x + min_x)) / 2.,
                        (y + settings.print_y - (max_y + min_y)) / 2.,
                        -min_z,
                    )
                }
            };

            let trans_str = serde_json::to_string(&transform).unwrap();

            println!("Using Transform {}", trans_str);

            for vert in vertices.iter_mut() {
                *vert = &transform * *vert;
            }

            (vertices, triangles)
        })
        .collect();

    println!("Creating Towers");
    let towers : Vec<TriangleTower>= converted_inputs.into_iter().map(|( vertices, triangles)|{
        if let Ok(tower) = TriangleTower::from_triangles_and_vertices(&triangles, vertices){
            tower
        }
        else{
            println!("Error Creating Tower. Model most likely needs repair. Please Repair and run again.");
            std::process::exit(-1);
        }
    }).collect();

    println!("Slicing");

    let mut objects: Vec<Object> = towers.into_iter().map(|tower| {

        let mut tower_iter = TriangleTowerIterator::new(&tower);

        let mut layer = 0.0;

        let mut first_layer = true;

        let slices: Vec<_> = std::iter::repeat(())
            .map(|_| {
                //Advance to the correct height
                let layer_height = if first_layer {
                    settings.first_layer_height
                } else {
                    settings.layer_height
                };

                layer += layer_height / 2.0;
                tower_iter.advance_to_height(layer).expect("Error Creating Tower. Model most likely needs repair. Please Repair and run again.");
                layer += layer_height / 2.0;

                first_layer = false;

                //Get the ordered lists of points
                (layer, tower_iter.get_points())
            })
            .take_while(|(_, layer_loops)| !layer_loops.is_empty())
            .map(|(l, layer_loops)| {
                //Add this slice to the
                let slice = Slice::from_multiple_point_loop(
                    layer_loops
                        .iter()
                        .map(|verts| {
                            verts
                                .iter()
                                .map(|v| Coordinate { x: v.x, y: v.y })
                                .collect::<Vec<Coordinate<f64>>>()
                        })
                        .collect(),
                );
                (l, slice)
            })
            .collect();

        Object{layers:slices}
    }).collect();

    println!("Generating Moves");

    //Handle Perimeters
    if let Some(skirt) = &settings.skirt {
        println!("Generating Moves: Skirt");
        let convex_hull = objects
            .iter()
            .map(|object| {
                object
                    .layers
                    .iter()
                    .take(skirt.layers)
                    .map(|m| m.1.get_entire_slice_polygon())
            })
            .flatten()
            .fold(
                objects
                    .get(0)
                    .expect("Needs an object")
                    .layers
                    .get(0)
                    .unwrap()
                    .1
                    .get_entire_slice_polygon()
                    .clone(),
                |a, b| a.union(b, 100000.0),
            )
            .convex_hull();

        //Add to first object
        objects
            .get_mut(0)
            .expect("Needs an object")
            .layers
            .iter_mut()
            .take(skirt.layers)
            .enumerate()
            .for_each(|(layer_num, (_layer, slice))| {
                slice.generate_skirt(&convex_hull, &settings.get_layer_settings(layer_num), skirt)
            })
    }

    objects.par_iter_mut().for_each(|object| {
        let slices = &mut object.layers;

        let slice_count = slices.len();

        //Handle Perimeters
        println!("Generating Moves: Perimeters");
        slices
            .par_iter_mut()
            .enumerate()
            .for_each(|(layer_num, (_layer, slice))| {
                slice.slice_perimeters_into_chains(
                    &settings.get_layer_settings(layer_num),
                    settings.number_of_perimeters,
                );
            });

        println!("Generating Moves: Bridging");
        (1..slices.len()).into_iter().for_each(|q| {
            let below = slices[q - 1].1.get_entire_slice_polygon().clone();

            slices[q]
                .1
                .fill_solid_bridge_area(&below, &settings.get_layer_settings(q));
        });

        println!("Generating Moves: Top Layer");
        (0..slices.len() - 1).into_iter().for_each(|q| {
            let above = slices[q + 1].1.get_entire_slice_polygon().clone();

            slices[q]
                .1
                .fill_solid_top_layer(&above, &settings.get_layer_settings(q), q);
        });
        //Combine layer to form support

        println!("Generating Moves: Above and below support");

        let layers = 3;
        (layers..slices.len() - layers).into_iter().for_each(|q| {
            let below = slices[(q - layers + 1)..q]
                .iter()
                .map(|m| m.1.get_entire_slice_polygon())
                .fold(
                    slices
                        .get(q - layers)
                        .unwrap()
                        .1
                        .get_entire_slice_polygon()
                        .clone(),
                    |a, b| a.intersection(b, 100000.0),
                );
            let above = slices[q + 2..q + layers]
                .iter()
                .map(|m| m.1.get_entire_slice_polygon())
                .fold(
                    slices
                        .get(q + 1)
                        .unwrap()
                        .1
                        .get_entire_slice_polygon()
                        .clone(),
                    |a, b| a.intersection(b, 100000.0),
                );
            let intersection = below.intersection(&above, 100000.0);

            slices.get_mut(q).unwrap().1.fill_solid_subtracted_area(
                &intersection,
                &settings.get_layer_settings(q),
                q,
            )
        });

        println!("Generating Moves: Fill Areas");
        //Fill all remaining areas
        slices
            .par_iter_mut()
            .enumerate()
            .for_each(|(layer_num, (_layer, slice))| {
                slice.fill_remaining_area(
                    &settings.get_layer_settings(layer_num),
                    layer_num < 3 || layer_num + 3 + 1 > slice_count,
                    layer_num,
                );
            });
    });

    println!("Convert into Commnds");
    let mut layer_moves: Vec<(f64, Vec<Command>)> = objects
        .into_iter()
        .enumerate()
        .map(|(object_num, object)| {
            let mut last_layer = 0.0;

            object
                .layers
                .into_iter()
                .enumerate()
                .map(|(layer_num, (layer, mut slice))| {
                    let mut moves = vec![];
                    moves.push(Command::ChangeObject { object: object_num });
                    moves.push(Command::LayerChange { z: layer });
                    moves.push(Command::SetState {
                        new_state: StateChange {
                            extruder_temp: Some(if layer_num == 0 {
                                settings.filament.first_layer_extruder_temp
                            } else {
                                settings.filament.extruder_temp
                            }),
                            bed_temp: Some(if layer_num == 0 {
                                settings.filament.first_layer_bed_temp
                            } else {
                                settings.filament.bed_temp
                            }),
                            fan_speed: Some(if layer_num < settings.fan.disable_fan_for_layers {
                                0.0
                            } else {
                                settings.fan.fan_speed
                            }),
                            movement_speed: None,
                            retract: None,
                        },
                    });
                    slice.slice_into_commands(
                        &settings.get_layer_settings(layer_num),
                        &mut moves,
                        layer - last_layer,
                    );

                    last_layer = layer;
                    (layer, moves)
                })
                .collect::<Vec<(f64, Vec<Command>)>>()
        })
        .map(|a| a.into_iter())
        .flatten()
        .collect();

    layer_moves.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

    let mut moves: Vec<_> = layer_moves
        .into_iter()
        .map(|(_, layer_moves)| layer_moves)
        .flatten()
        .collect();

    println!("Optimizing {} Moves", moves.len());
    optimize_commands(&mut moves, &settings);

    let mut layer_height = 0.0;
    //Slow down on small layers
    let mut current_speed = 0.0;
    let mut current_pos = Coordinate { x: 0.0, y: 0.0 };

    {
        let layers: Vec<(HashMap<OrderedFloat<f64>, f64>, f64, usize, usize)> = moves
            .iter()
            .enumerate()
            .batching(|it| {
                //map from speed to length at that speed
                let mut map: HashMap<OrderedFloat<f64>, f64> = HashMap::new();
                let mut non_move_time = 0.0;

                let start_z_height = layer_height;
                let mut return_none = false;

                let mut start_index = None;
                let mut end_index = 0;
                while layer_height == start_z_height && !return_none {
                    if let Some((index, cmd)) = it.next() {
                        start_index = start_index.or(Some(index));
                        end_index = index;
                        match cmd {
                            Command::MoveTo { end } => {
                                let x_diff = end.x - current_pos.x;
                                let y_diff = end.y - current_pos.y;
                                let d = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();
                                current_pos = *end;
                                if current_speed != 0.0 {
                                    non_move_time += d / current_speed;
                                }
                            }
                            Command::MoveAndExtrude {
                                start,
                                end,
                                width,
                                thickness,
                            } => {
                                let x_diff = end.x - start.x;
                                let y_diff = end.y - start.y;
                                let d = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();
                                current_pos = *end;
                                *map.entry(OrderedFloat(current_speed)).or_insert(0.0) += d;
                            }
                            Command::SetState { new_state } => {
                                if let Some(speed) = new_state.movement_speed {
                                    current_speed = speed
                                }
                                if new_state.retract.is_some() {
                                    non_move_time +=
                                        settings.retract_length / settings.retract_speed;
                                    non_move_time +=
                                        settings.retract_lift_z / settings.travel_speed;
                                }
                            }
                            Command::Delay { msec } => {
                                non_move_time += *msec as f64 / 1000.0;
                            }
                            Command::Arc { .. } => {
                                unimplemented!()
                            }
                            Command::LayerChange { z } => {
                                layer_height = *z;
                            }
                            Command::NoAction | Command::ChangeObject { .. } => {}
                        }
                    } else {
                        return_none = true;
                    }
                }

                if return_none {
                    if map.is_empty() {
                        None
                    } else {
                        Some((map, non_move_time, start_index.unwrap(), end_index))
                    }
                } else {
                    Some((map, non_move_time, start_index.unwrap(), end_index))
                }
            })
            .collect();

        layers
            .into_iter()
            .filter_map(|(map, time, start, end)| {
                let mut total_time = time
                    + map
                        .iter()
                        .map(|(speed, len)| len / speed.into_inner())
                        .sum::<f64>();

                let min_time = settings.fan.slow_down_threshold;
                if total_time < min_time && !map.is_empty() {
                    let mut sorted = map.into_iter().collect::<Vec<(OrderedFloat<f64>, f64)>>();
                    sorted.sort_by(|a, b| a.0.cmp(&b.0));

                    let max_speed: f64 ;
                    loop {
                        let (speed, len) = sorted.pop().unwrap();
                        let (top_speed, _) =
                            sorted.last().unwrap_or(&(OrderedFloat(0.000001), 0.0));

                        if min_time - total_time
                            < (len / top_speed.into_inner()) - (len / speed.into_inner())
                        {
                            let second = min_time - total_time;
                            max_speed =
                                (len * speed.into_inner()) / (len + (second * speed.into_inner()));
                            break;
                        } else {
                            total_time +=
                                (len / top_speed.into_inner()) - (len / speed.into_inner());
                            //println!("tt: {:.5}", total_time);
                        }
                    }
                    Some((max_speed, start, end))
                } else {
                    None
                }
            })
            .for_each(|(max_speed, start, end)| {
                for cmd in &mut moves[start..end] {
                    if let Command::SetState { new_state } = cmd {
                        if let Some(speed) = &mut new_state.movement_speed {
                            if *speed != settings.travel_speed {
                                *speed = speed.min(max_speed).max(settings.fan.min_print_speed);
                            }
                        }
                    }
                }
            });
    }

    //Convert all commands into
    let mut plastic_used = 0.0;
    let mut total_time = 0.0;
    let mut current_speed = 0.0;
    let mut current_pos = Coordinate { x: 0.0, y: 0.0 };

    for cmd in &moves {
        match cmd {
            Command::MoveTo { end } => {
                let x_diff = end.x - current_pos.x;
                let y_diff = end.y - current_pos.y;
                let d = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();
                current_pos = *end;
                if current_speed != 0.0 {
                    total_time += d / current_speed;
                }
            }
            Command::MoveAndExtrude {
                start,
                end,
                width,
                thickness,
            } => {
                let x_diff = end.x - start.x;
                let y_diff = end.y - start.y;
                let d = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();
                current_pos = *end;
                total_time += d / current_speed;

                plastic_used += width * thickness * d;
            }
            Command::SetState { new_state } => {
                if let Some(speed) = new_state.movement_speed {
                    current_speed = speed
                }
                if new_state.retract.is_some() {
                    total_time += settings.retract_length / settings.retract_speed;
                    total_time += settings.retract_lift_z / settings.travel_speed;
                }
            }
            Command::Delay { msec } => {
                total_time += *msec as f64 / 1000.0;
            }
            Command::Arc { .. } => {
                unimplemented!()
            }
            Command::NoAction | Command::LayerChange { .. } | Command::ChangeObject { .. } => {}
        }
    }

    let total_time = total_time.floor() as u32;

    println!(
        "Total Time: {} hours {} minutes {:.3} seconds",
        total_time / 3600,
        (total_time % 3600) / 60,
        total_time % 60
    );
    println!("Total Filament Volume: {:.3} cm^3", plastic_used / 1000.0);
    println!(
        "Total Filament Mass: {:.3} grams",
        (plastic_used / 1000.0) * settings.filament.density
    );
    println!(
        "Total Filament Cost: {:.2} $",
        (((plastic_used / 1000.0) * settings.filament.density) / 1000.0) * settings.filament.cost
    );

    //Output the GCode
    if let Some(file_path) = matches.value_of("OUTPUT") {
        //Output to file
        println!("Converting {} Moves", moves.len());
        convert(
            &moves,
            settings,
            &mut File::create(file_path).expect("File not Found"),
        )
        .unwrap();
    } else {
        //Output to stdout
        let stdout = std::io::stdout();
        println!("Converting {} Moves", moves.len());
        convert(&moves, settings, &mut stdout.lock()).unwrap();
    };
}

fn convert(
    cmds: &[Command],
    settings: Settings,
    write: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_z = 0.0;

    let mut start = settings.starting_gcode.clone();
    let mut write_buf = BufWriter::new(write);
    start = start.replace(
        "[First Layer Extruder Temp]",
        &format!("{:.1}", settings.filament.first_layer_extruder_temp),
    );
    start = start.replace(
        "[First Layer Bed Temp]",
        &format!("{:.1}", settings.filament.first_layer_bed_temp),
    );

    writeln!(write_buf, "{}", start)?;

    for cmd in cmds {
        match cmd {
            Command::MoveTo { end, .. } => writeln!(write_buf, "G1 X{:.5} Y{:.5}", end.x, end.y)?,
            Command::MoveAndExtrude {
                start,
                end,
                width,
                thickness,
            } => {
                let x_diff = end.x - start.x;
                let y_diff = end.y - start.y;
                let length = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();

                //let extrusion_width = width + (thickness * (1.0 - std::f64::consts::FRAC_PI_4));

                let extrusion_volume = (((width - thickness) * thickness)
                    + (std::f64::consts::PI * (thickness / 2.0) * (thickness / 2.0)))
                    * length;
                let filament_area = (std::f64::consts::PI
                    * settings.filament.diameter
                    * settings.filament.diameter)
                    / 4.0;
                let extrude = extrusion_volume / filament_area;

                writeln!(write_buf, "G1 X{:.5} Y{:.5} E{:.5}", end.x, end.y, extrude)?;
            }
            Command::SetState { new_state } => {
                match new_state.retract {
                    None => {}
                    Some(true) => {
                        //retract
                        writeln!(
                            write_buf,
                            "G1 E{:.5} F{:.5}; Retract or unretract",
                            -settings.retract_length,
                            60.0 * settings.retract_speed,
                        )?;

                        writeln!(
                            write_buf,
                            "G1 Z{:.5} F{:.5}; z Lift",
                            current_z + settings.retract_lift_z,
                            60.0 * settings.travel_speed,
                        )?;
                    }
                    Some(false) => {
                        //unretract
                        writeln!(write_buf, "G1 Z{:.5}; z unlift", current_z,)?;
                        writeln!(
                            write_buf,
                            "G1 E{:.5} F{:.5}; Retract or unretract",
                            settings.retract_length,
                            60.0 * settings.retract_speed,
                        )?;
                    }
                }

                if let Some(speed) = new_state.movement_speed {
                    writeln!(write_buf, "G1 F{:.5}", speed * 60.0)?;
                }
                if let Some(ext_temp) = new_state.extruder_temp {
                    writeln!(write_buf, "M104 S{:.1} ; set extruder temp", ext_temp)?;
                }
                if let Some(bed_temp) = new_state.bed_temp {
                    writeln!(write_buf, "M140 S{:.1} ; set bed temp", bed_temp)?;
                }
                if let Some(fan_speed) = new_state.fan_speed {
                    writeln!(
                        write_buf,
                        "M106 S{} ; set fan speed",
                        (2.550 * fan_speed).round() as usize
                    )?;
                }
            }
            Command::LayerChange { z } => {
                current_z = *z;
                writeln!(write_buf, "G1 Z{:.5}", z)?;
            }
            Command::Delay { msec } => {
                writeln!(write_buf, "G4 P{:.5}", msec)?;
            }
            Command::Arc {
                start,
                end,
                center,
                clockwise,
                width,
                thickness,
            } => {
                let x_diff = end.x - start.x;
                let y_diff = end.y - start.y;
                let cord_length = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();
                let x_diff_r = end.x - center.x;
                let y_diff_r = end.y - center.y;
                let radius = ((x_diff_r * x_diff_r) + (y_diff_r * y_diff_r)).sqrt();

                //Divide the chord length by double the radius.
                let t = cord_length / (2.0 * radius);
                //println!("{}",t);
                //Find the inverse sine of the result (in radians).
                //Double the result of the inverse sine to get the central angle in radians.
                let central = t.asin() * 2.0;
                //Once you have the central angle in radians, multiply it by the radius to get the arc length.
                let extrusion_length = central * radius;

                //println!("{}",extrusion_length);
                let extrude = (4.0 * thickness * width * extrusion_length)
                    / (std::f64::consts::PI
                        * settings.filament.diameter
                        * settings.filament.diameter);
                writeln!(
                    write_buf,
                    "{} X{:.5} Y{:.5} I{:.5} J{:.5} E{:.5}",
                    if *clockwise { "G2" } else { "G3" },
                    end.x,
                    end.y,
                    center.x,
                    center.y,
                    extrude
                )?;
            }
            Command::ChangeObject { object } => {
                writeln!(write_buf, "; Change Object to {}", object)?;
            }
            Command::NoAction => {
                panic!("Converter reached a No Action Command, Optimization Failure")
            }
        }
    }

    let end = settings.ending_gcode;

    writeln!(write_buf, "{}", end)?;

    write_buf
        .flush()
        .expect("File Closed Before CLosed. Gcode invalid.");

    Ok(())
}
