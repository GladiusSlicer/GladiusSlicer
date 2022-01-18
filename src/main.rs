#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use clap::{load_yaml, App};
use gladius_shared::loader::*;
use gladius_shared::types::*;

use crate::plotter::convert_objects_into_moves;
use crate::tower::*;
use geo::*;
use gladius_shared::settings::{PartialSettings, Settings};
use std::fs::File;

use std::ffi::OsStr;
use std::path::Path;

use crate::calculation::calculate_values;
use crate::command_pass::{CommandPass, OptimizePass, SlowDownLayerPass};
use crate::coverter::*;
use crate::input::files_input;
use crate::plotter::polygon_operations::PolygonOperations;
use crate::slice_pass::*;
use crate::slicing::*;
use gladius_shared::error::SlicerErrors;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use std::collections::HashMap;
use log::{debug, info, LevelFilter};
use simple_logger::SimpleLogger;

mod calculation;
mod command_pass;
mod coverter;
mod input;
mod optimizer;
mod plotter;
mod slice_pass;
mod slicing;
mod tower;
mod utils;

fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

        // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    match matches.occurrences_of("VERBOSE") {
        0 => SimpleLogger::new().with_level(LevelFilter::Error ).init().unwrap(),
        1 => SimpleLogger::new().with_level(LevelFilter::Warn ).init().unwrap(),
        2 => SimpleLogger::new().with_level(LevelFilter::Info ).init().unwrap(),
        3 => SimpleLogger::new().with_level(LevelFilter::Debug ).init().unwrap(),
        4 | _ => SimpleLogger::new().with_level(LevelFilter::Trace ).init().unwrap(),
    }

    info!("Loading Inputs");
    let (models, settings) = files_input(
        matches.value_of("SETTINGS"),
        matches
            .values_of("INPUT")
            .map(|values| values.map(|v| v.to_string()).collect()),
    );

    info!("Creating Towers");

    let towers: Vec<TriangleTower> = create_towers(&models);

    info!("Slicing");

    let mut objects = slice(&towers, &settings);

    info!("Generating Moves");

    //Creates Support Towers
    SupportTowerPass::pass(&mut objects, &settings);

    //Adds a skirt
    SkirtPass::pass(&mut objects, &settings);

    //Adds a brim
    BrimPass::pass(&mut objects, &settings);

    objects.par_iter_mut().for_each(|object| {
        let slices = &mut object.layers;

        //Shrink layer
        ShrinkPass::pass(slices, &settings);

        //Handle Perimeters
        PerimeterPass::pass(slices, &settings);

        //Handle Bridging
        BridgingPass::pass(slices, &settings);

        //Handle Top Layer
        TopLayerPass::pass(slices, &settings);

        //Handle Top And Bottom Layers
        TopAndBottomLayersPass::pass(slices, &settings);

        //Handle Support
        SupportPass::pass(slices, &settings);

        //Lightning Infill
        LightningFillPass::pass(slices, &settings);

        //Fill Remaining areas
        FillAreaPass::pass(slices, &settings);

        //Order the move chains
        OrderPass::pass(slices, &settings);
    });

    let mut moves = convert_objects_into_moves(objects, &settings);

    debug!("Optimizing {} Moves", moves.len());
    OptimizePass::pass(&mut moves, &settings);

    SlowDownLayerPass::pass(&mut moves, &settings);

    let cv = calculate_values(&moves, &settings);

    let total_time = cv.total_time.floor() as u32;

    info!(
        "Total Time: {} hours {} minutes {:.3} seconds",
        total_time / 3600,
        (total_time % 3600) / 60,
        total_time % 60
    );
    info!(
        "Total Filament Volume: {:.3} cm^3",
        cv.plastic_used / 1000.0
    );
    info!(
        "Total Filament Mass: {:.3} grams",
        (cv.plastic_used / 1000.0) * settings.filament.density
    );
    info!(
        "Total Filament Cost: {:.2} $",
        (((cv.plastic_used / 1000.0) * settings.filament.density) / 1000.0)
            * settings.filament.cost
    );

    //Output the GCode
    if let Some(file_path) = matches.value_of("OUTPUT") {
        //Output to file
        debug!("Converting {} Moves", moves.len());
        convert(
            &moves,
            settings,
            &mut File::create(file_path).expect("File not Found"),
        )
        .unwrap();
    } else {
        //Output to stdout
        let stdout = std::io::stdout();
        debug!("Converting {} Moves", moves.len());
        convert(&moves, settings, &mut stdout.lock()).unwrap();
    };
}
