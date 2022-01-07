use crate::loader::*;
use crate::types::*;
use clap::{load_yaml, App};

use crate::plotter::{convert_objects_into_moves, Slice};
use crate::settings::{PartialSettings, Settings};
use crate::tower::*;
use geo::*;
use std::fs::File;

use std::ffi::OsStr;
use std::path::Path;

use crate::calculation::calculate_values;
use crate::command_pass::{CommandPass, OptimizePass, SlowDownLayerPass};
use crate::coverter::*;
use crate::error::SlicerErrors;
use crate::input::files_input;
use crate::plotter::polygon_operations::PolygonOperations;
use crate::slice_pass::*;
use crate::slicing::*;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use std::collections::HashMap;

mod calculation;
mod command_pass;
mod coverter;
mod error;
mod input;
mod loader;
mod optimizer;
mod plotter;
mod settings;
mod slice_pass;
mod slicing;
mod tower;
mod types;

fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    println!("Loading Inputs");
    let (models, settings) = files_input(
        matches.value_of("SETTINGS"),
        matches
            .values_of("INPUT")
            .map(|values| values.map(|v| v.to_string()).collect()),
    );

    println!("Creating Towers");

    let towers: Vec<TriangleTower> = create_towers(&models);

    println!("Slicing");

    let mut objects = slice(&towers, &settings);

    println!("Generating Moves");

    //Creates Support Towers
    SupportTowerPass::pass(&mut objects, &settings);

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

    let mut moves = convert_objects_into_moves(objects, &settings);

    println!("Optimizing {} Moves", moves.len());
    OptimizePass::pass(&mut moves, &settings);

    SlowDownLayerPass::pass(&mut moves, &settings);

    let cv = calculate_values(&moves, &settings);

    let total_time = cv.total_time.floor() as u32;

    println!(
        "Total Time: {} hours {} minutes {:.3} seconds",
        total_time / 3600,
        (total_time % 3600) / 60,
        total_time % 60
    );
    println!(
        "Total Filament Volume: {:.3} cm^3",
        cv.plastic_used / 1000.0
    );
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
