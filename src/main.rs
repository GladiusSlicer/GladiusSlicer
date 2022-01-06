use crate::loader::*;
use crate::types::*;
use clap::{load_yaml, App};
use log::LevelFilter;
use simple_logger::SimpleLogger;

use crate::optimizer::optimize_commands;
use crate::plotter::Slice;
use crate::settings::{PartialSettings, Settings};
use crate::tower::*;
use geo::*;
use std::fs::File;

use std::ffi::OsStr;
use std::path::Path;

use crate::coverter::*;
use crate::error::SlicerErrors;
use crate::plotter::polygon_operations::PolygonOperations;
use crate::slice_pass::*;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use std::collections::HashMap;

mod coverter;
mod error;
mod loader;
mod optimizer;
mod plotter;
mod settings;
mod slice_pass;
mod tower;
mod types;

fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let settings_res: Result<Settings, SlicerErrors> = {
        let settings_path = matches.value_of("SETTINGS");
        if let Some(str) = settings_path {
            load_settings(str)
        } else {
            Ok(Settings::default())
        }
    };

    let settings = {
        match settings_res {
            Ok(settings) => settings,
            Err(err) => {
                err.show_error_message();
                std::process::exit(-1);
            }
        }
    };

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
        .unwrap_or_else(|| {
            SlicerErrors::NoInputProvided.show_error_message();
            std::process::exit(-1);
        })
        .map(|value| {
            let obj: InputObject = deser_hjson::from_str(value).unwrap_or_else(|_| {
                SlicerErrors::InputMisformat.show_error_message();
                std::process::exit(-1);
            });
            obj
        })
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

            let models = match loader.load(model_path.to_str().unwrap()) {
                Ok(v) => v,
                Err(err) => {
                    err.show_error_message();
                    std::process::exit(-1);
                }
            };

            let (x, y) = match object {
                InputObject::AutoTranslate(_, x, y) => (x, y),
                _ => (0.0, 0.0),
            };

            let transform = match object {
                InputObject::Raw(_, transform) => transform,
                InputObject::Auto(_) | InputObject::AutoTranslate(_, _, _) => {
                    let (min_x, max_x, min_y, max_y, min_z) =
                        models.iter().map(|(v, _t)| v.iter()).flatten().fold(
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

            models.into_iter().map(move |(mut v, t)| {
                for vert in v.iter_mut() {
                    *vert = &transform * *vert;
                }

                (v, t)
            })
        })
        .flatten()
        .collect();

    println!("Creating Towers");
    let towers: Vec<TriangleTower> = converted_inputs
        .into_iter()
        .map(|(vertices, triangles)| {
            match TriangleTower::from_triangles_and_vertices(&triangles, vertices) {
                Ok(tower) => tower,
                Err(err) => {
                    err.show_error_message();
                    std::process::exit(-1);
                }
            }
        })
        .collect();

    println!("Slicing");

    let mut objects: Vec<Object> = towers.into_iter().map(|tower| {
        let mut tower_iter = TriangleTowerIterator::new(&tower);

        let mut layer = 0.0;

        let mut first_layer = true;

        let slices: Vec<_> = std::iter::repeat(())
            .enumerate()
            .map(|(layer_count, _)| {
                //Advance to the correct height
                let layer_height = settings.get_layer_settings(layer_count, layer).layer_height;

                let bottom_height = layer;
                layer += layer_height / 2.0;
                tower_iter.advance_to_height(layer).expect("Error Creating Tower. Model most likely needs repair. Please Repair and run again.");
                layer += layer_height / 2.0;

                let top_height = layer;

                first_layer = false;

                //Get the ordered lists of points
                (bottom_height, top_height, tower_iter.get_points())
            })
            .take_while(|(_, _, layer_loops)| !layer_loops.is_empty())
            .enumerate()
            .map(|(count, (bot, top, layer_loops))| {
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
                    bot,
                    top,
                    count,
                    &settings
                );
                slice
            })
            .collect();

        Object { layers: slices }
    }).collect();

    println!("Generating Moves");

    //Adds a skirt
    SkirtPass::pass(&mut objects, &settings);

    //Adds a brim
    BrimPass::pass(&mut objects, &settings);

    objects.par_iter_mut().for_each(|object| {
        let mut slices = &mut object.layers;

        //Handle Perimeters
        PerimeterPass::pass(&mut slices, &settings);

        //Handle Bridging
        BridgingPass::pass(&mut slices, &settings);

        //Handle Top Layer
        TopLayerPass::pass(&mut slices, &settings);

        //Handle Top And Bottom Layers
        TopAndBottomLayersPass::pass(&mut slices, &settings);

        //Handle Support
        SupportPass::pass(&mut slices, &settings);

        //Fill Remaining areas
        FillAreaPass::pass(&mut slices, &settings);
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
                .map(|(layer_num, mut slice)| {
                    let layer_settings = settings.get_layer_settings(layer_num, slice.top_height);
                    let mut moves = vec![];
                    moves.push(Command::ChangeObject { object: object_num });
                    moves.push(Command::LayerChange {
                        z: slice.top_height,
                    });
                    moves.push(Command::SetState {
                        new_state: StateChange {
                            extruder_temp: Some(layer_settings.extruder_temp),
                            bed_temp: Some(layer_settings.bed_temp),
                            fan_speed: Some(if layer_num < settings.fan.disable_fan_for_layers {
                                0.0
                            } else {
                                settings.fan.fan_speed
                            }),
                            movement_speed: None,
                            acceleration: None,
                            retract: None,
                        },
                    });
                    slice.slice_into_commands(&mut moves, slice.top_height - last_layer);

                    last_layer = slice.top_height;
                    (slice.top_height, moves)
                })
                .collect::<Vec<(f64, Vec<Command>)>>()
        })
        .map(|a| a.into_iter())
        .flatten()
        .collect();

    layer_moves
        .sort_by(|(a, _), (b, _)| a.partial_cmp(b).expect("No NAN layer heights are allowed"));

    let mut moves: Vec<_> = layer_moves
        .into_iter()
        .map(|(_, layer_moves)| layer_moves)
        .flatten()
        .collect();

    println!("Optimizing {} Moves", moves.len());
    optimize_commands(&mut moves, &settings);

    {
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
                                    width: _width,
                                    thickness: _thickness,
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
                                            settings.retract_lift_z / settings.speed.travel;
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

                        let max_speed: f64;
                        loop {
                            let (speed, len) = sorted.pop().unwrap();
                            let (top_speed, _) =
                                sorted.last().unwrap_or(&(OrderedFloat(0.000001), 0.0));

                            if min_time - total_time
                                < (len / top_speed.into_inner()) - (len / speed.into_inner())
                            {
                                let second = min_time - total_time;
                                max_speed = (len * speed.into_inner())
                                    / (len + (second * speed.into_inner()));
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
                                if *speed != settings.speed.travel {
                                    *speed = speed.min(max_speed).max(settings.fan.min_print_speed);
                                }
                            }
                        }
                    }
                });
        }
    }

    {
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
                        total_time += settings.retract_lift_z / settings.speed.travel;
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
            (((plastic_used / 1000.0) * settings.filament.density) / 1000.0)
                * settings.filament.cost
        );
    }

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

fn load_settings(filepath: &str) -> Result<Settings, SlicerErrors> {
    let settings_data =
        std::fs::read_to_string(filepath).map_err(|_| SlicerErrors::SettingsFileNotFound {
            filepath: filepath.to_string(),
        })?;
    let partial_settings: PartialSettings =
        deser_hjson::from_str(&settings_data).map_err(|_| SlicerErrors::SettingsFileMisformat {
            filepath: filepath.to_string(),
        })?;
    let settings = partial_settings.get_settings().map_err(|err| {
        SlicerErrors::SettingsFileMissingSettings {
            missing_setting: err,
        }
    })?;
    Ok(settings)
}
