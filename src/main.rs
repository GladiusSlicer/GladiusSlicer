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
use crate::slicing::slice;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use std::collections::HashMap;
use crate::calculation::calculate_values;
use crate::input::files_input;

mod coverter;
mod error;
mod loader;
mod optimizer;
mod plotter;
mod settings;
mod slice_pass;
mod slicing;
mod tower;
mod types;
mod input;
mod calculation;

fn main() {

    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();



    println!("Loading Inputs");
    let (models,settings) = files_input(matches.value_of("SETTINGS"),matches.values_of("INPUT").map(|values| values.map(|v| v.to_string()).collect()));

    println!("Creating Towers");

    let towers: Vec<TriangleTower> = create_towers(&models);

    println!("Slicing");

    let mut objects = slice(&towers,&settings);

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

        //Order the move chains
        OrderPass::pass(&mut slices, &settings);
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

    let cv = calculate_values(&moves,&settings);

    let total_time = cv.total_time.floor() as u32;

    println!(
        "Total Time: {} hours {} minutes {:.3} seconds",
        total_time / 3600,
        (total_time % 3600) / 60,
        total_time % 60
    );
    println!("Total Filament Volume: {:.3} cm^3", cv.plastic_used / 1000.0);
    println!(
        "Total Filament Mass: {:.3} grams",
        (cv.plastic_used / 1000.0) * settings.filament.density
    );
    println!(
        "Total Filament Cost: {:.2} $",
        (((cv.plastic_used / 1000.0) * settings.filament.density) / 1000.0)
            * settings.filament.cost
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
