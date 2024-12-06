#![deny(clippy::unwrap_used)]
#![warn(clippy::all, clippy::perf, clippy::missing_const_for_fn)]

use clap::Parser;
use gladius_shared::loader::{Loader, STLLoader, ThreeMFLoader};
use gladius_shared::types::*;
use input::load_settings;
use utils::{DisplayType, StateContext};

use crate::plotter::convert_objects_into_moves;
use crate::tower::{create_towers, TriangleTower, TriangleTowerIterator};
use geo::{
    coordinate_position, Closest, ClosestPoint, Contains, Coord, CoordinatePosition, GeoFloat,
    Line, MultiPolygon, Point,
};
use gladius_shared::settings::{PartialSettingsFile, Settings, SettingsValidationResult};
use std::fs::File;

use std::ffi::OsStr;
use std::path::Path;

use crate::bounds_checking::{check_model_bounds, check_moves_bounds};
use crate::calculation::calculate_values;
use crate::command_pass::{CommandPass, OptimizePass, SlowDownLayerPass};
use crate::converter::convert;
use crate::plotter::polygon_operations::PolygonOperations;
use crate::slice_pass::*;
use crate::slicing::slice;
use crate::utils::{
    send_error_message, send_warning_message, show_error_message, show_warning_message,
    state_update,
};
use gladius_shared::error::SlicerErrors;
use gladius_shared::messages::Message;
use itertools::Itertools;
use log::{debug, info, LevelFilter};
use ordered_float::OrderedFloat;
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::io::BufWriter;

mod bounds_checking;
mod calculation;
mod command_pass;
mod converter;
mod input;
mod optimizer;
mod plotter;
mod slice_pass;
mod slicing;
mod tower;
mod utils;
#[cfg(test)]
mod test;

pub static PLANE_NORMAL: std::sync::OnceLock<Vertex> = std::sync::OnceLock::new();

#[derive(Parser)]
#[command(author, version, about,
    long_about = "An *In-Progress* Slicer for FDM 3D printing written in Rust with a focus on customization and modularity."
)]
#[clap(group(
    clap::ArgGroup::new("settings-group")
        .required(true)
        .args(&["settings_file_path", "settings_json"]),
))]
struct Args {
    #[arg(
        required = true,
        help = "The input files and there translations.
By default it takes a list of json strings that represents how the models should be loaded and translated.
See simple_input for an alternative command."
    )]
    input: Vec<String>,

    #[arg(short = 'o', help = "Sets the output dir")]
    output: Option<String>,

    #[arg(short = 'v', action = clap::ArgAction::Count, conflicts_with = "message", help = "Sets the level of verbosity")]
    verbose: u8,

    #[arg(short = 's', help = "Sets the settings file to use")]
    settings_file_path: Option<String>,
    #[arg(short = 'S', help = "The contents of a json settings file.")]
    settings_json: Option<String>,
    #[arg(short = 'm', help = "Use the Message System (useful for interprocess communication)")]
    message: bool,

    #[arg(
        long = "print_settings",
        help = "Print the final combined settings out to Stdout and Terminate. Verbose level 4 will print but continue."
    )]
    print_settings: bool,
    #[arg(
        long = "simple_input",
        help = "The input should only be a list of files that will be auto translated to the center of the build plate."
    )]
    simple_input: bool,
    #[arg(
        short = 'j',
        help = "Sets the number of threads to use in the thread pool (defaults to number of CPUs)"
    )]
    thread_count: Option<usize>,
}

fn main() {
    #[cfg(feature = "json_schema_gen")]
    // export json schema for settings
    Settings::gen_schema(Path::new("settings/")).expect("The programme should exit if this fails");

    // The YAML file is found relative to the current file, similar to how modules are found
    let args: Args = Args::parse();

    // set number of cores for rayon
    if let Some(number_of_threads) = args.thread_count {
        rayon::ThreadPoolBuilder::new()
            .num_threads(number_of_threads)
            .build_global()
            .expect("Only call to build global");
    }

    let mut state_context = StateContext::new(if args.message {
        DisplayType::Message
    } else {
        DisplayType::StdOut
    });

    if !args.message {
        // Vary the output based on how many times the user used the "verbose" flag
        // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'

        SimpleLogger::new()
            .with_level(match args.verbose {
                0 => LevelFilter::Error,
                1 => LevelFilter::Warn,
                2 => LevelFilter::Info,
                3 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            })
            .init()
            .expect("Only Logger Setup");
    }

    state_update("Loading Inputs", &mut state_context);

    let settings_json = args.settings_json.unwrap_or_else(|| {
        handle_err_or_return(
            input::load_settings_json(
                args.settings_file_path
                    .as_deref()
                    .expect("CLAP should handle requiring a settings option to be Some"),
            ),
            &state_context,
        )
    });

    let settings = handle_err_or_return(
        load_settings(args.settings_file_path.as_deref(), &settings_json),
        &state_context,
    );

    // Set the plane normal
    PLANE_NORMAL.get_or_init(|| tower::angle_to_normal(settings.slice_angle.unwrap_or(0.0)));

    let models = handle_err_or_return(
        crate::input::load_models(Some(args.input), &settings, args.simple_input),
        &state_context,
    );
    if args.print_settings {
        for line in gladius_shared::settings::SettingsPrint::to_strings(&settings) {
            println!("{}", line);
        }
        std::process::exit(0);
    } else if log::log_enabled!(log::Level::Trace) {
        for line in gladius_shared::settings::SettingsPrint::to_strings(&settings) {
            log::trace!("{}", line);
        }
    }

    handle_err_or_return(check_model_bounds(&models, &settings), &state_context);

    handle_setting_validation(settings.validate_settings(), &state_context);

    state_update("Creating Towers", &mut state_context);

    let towers: Vec<TriangleTower<_>> = handle_err_or_return(
        create_towers::<tower::NormalVertex>(&models),
        &state_context,
    );

    state_update("Slicing", &mut state_context);

    let objects = handle_err_or_return(slice(towers, &settings), &state_context);

    state_update("Generating Moves", &mut state_context);

    let mut moves = handle_err_or_return(
        generate_moves(objects, &settings, &mut state_context),
        &state_context,
    );

    handle_err_or_return(check_moves_bounds(&moves, &settings), &state_context);

    state_update("Optimizing", &mut state_context);
    debug!("Optimizing {} Moves", moves.len());

    OptimizePass::pass(&mut moves, &settings);
    state_update("Slowing Layer Down", &mut state_context);

    SlowDownLayerPass::pass(&mut moves, &settings);

    if let DisplayType::Message = state_context.display_type {
        let message = Message::Commands(moves.clone());
        bincode::serialize_into(BufWriter::new(std::io::stdout()), &message)
            .expect("Write Limit should not be hit");
    }
    state_update("Calculate Values", &mut state_context);

    // Display info about the print
    print_info_message(&state_context, &moves, &settings);

    state_update("Outputting G-code", &mut state_context);

    // Output the GCode
    if let Some(file_path) = &args.output {
        // Output to file
        debug!("Converting {} Moves", moves.len());
        handle_err_or_return(
            convert(
                &moves,
                &settings,
                &mut handle_err_or_return(
                    File::create(file_path).map_err(|_| SlicerErrors::FileCreateError {
                        filepath: file_path.to_string(),
                    }),
                    &state_context,
                ),
            ),
            &state_context,
        );
    } else {
        match state_context.display_type {
            DisplayType::Message => {
                // Output as message
                let mut gcode: Vec<u8> = Vec::new();
                handle_err_or_return(convert(&moves, &settings, &mut gcode), &state_context);
                let message = Message::GCode(
                    String::from_utf8(gcode)
                        .expect("All write occur from write macro so should be utf-8"),
                );
                bincode::serialize_into(BufWriter::new(std::io::stdout()), &message)
                    .expect("Write Limit should not be hit");
            }
            DisplayType::StdOut => {
                // Output to stdout
                let stdout = std::io::stdout();
                let mut stdio_lock = stdout.lock();
                debug!("Converting {} Moves", moves.len());
                handle_err_or_return(convert(&moves, &settings, &mut stdio_lock), &state_context);
            }
        }
    };

    if let DisplayType::StdOut = state_context.display_type {
        info!(
            "Total slice time {} msec",
            state_context.get_total_elapsed_time().as_millis()
        );
    }
}

/// Display info about the print; time and filament info
fn print_info_message(state_context: &StateContext, moves: &[Command], settings: &Settings) {
    let cv = calculate_values(moves, settings);

    match state_context.display_type {
        DisplayType::Message => {
            let message = Message::CalculatedValues(cv);
            bincode::serialize_into(BufWriter::new(std::io::stdout()), &message)
                .expect("Write Limit should not be hit");
        }
        DisplayType::StdOut => {
            let (hour, min, sec, _) = cv.get_hours_minutes_seconds_fract_time();

            info!(
                "Total Time: {} hours {} minutes {:.3} seconds",
                hour, min, sec
            );
            info!(
                "Total Filament Volume: {:.3} cm^3",
                cv.plastic_volume / 1000.0
            );
            info!("Total Filament Mass: {:.3} grams", cv.plastic_weight);
            info!(
                "Total Filament Length: {:.3} meters",
                cv.plastic_length / 1000.0
            );
            info!(
                "Total Filament Cost: ${:.2}",
                (((cv.plastic_volume / 1000.0) * settings.filament.density) / 1000.0)
                    * settings.filament.cost
            );
        }
    }
}

fn generate_moves(
    mut objects: Vec<Object>,
    settings: &Settings,
    state_context: &mut StateContext,
) -> Result<Vec<Command>, SlicerErrors> {
    // Creates Support Towers
    SupportTowerPass::pass(&mut objects, settings, state_context);

    // Adds a skirt
    SkirtPass::pass(&mut objects, settings, state_context);

    // Adds a brim
    BrimPass::pass(&mut objects, settings, state_context);

    let v: Result<Vec<()>, SlicerErrors> = objects
        .iter_mut()
        .map(|object| {
            let slices = &mut object.layers;

            // Shrink layer
            ShrinkPass::pass(slices, settings, state_context)?;

            // Handle Perimeters
            PerimeterPass::pass(slices, settings, state_context)?;

            // Handle Bridging
            BridgingPass::pass(slices, settings, state_context)?;

            // Handle Top Layer
            TopLayerPass::pass(slices, settings, state_context)?;

            // Handle Top And Bottom Layers
            TopAndBottomLayersPass::pass(slices, settings, state_context)?;

            // Handle Support
            SupportPass::pass(slices, settings, state_context)?;

            // Lightning Infill
            LightningFillPass::pass(slices, settings, state_context)?;

            // Fill Remaining areas
            FillAreaPass::pass(slices, settings, state_context)?;

            // Order the move chains
            OrderPass::pass(slices, settings, state_context)
        })
        .collect();

    v?;

    Ok(convert_objects_into_moves(objects, settings))
}

fn handle_err_or_return<T>(res: Result<T, SlicerErrors>, state_context: &StateContext) -> T {
    match res {
        Ok(data) => data,
        Err(slicer_error) => {
            match state_context.display_type {
                DisplayType::Message => send_error_message(slicer_error),
                DisplayType::StdOut => show_error_message(&slicer_error),
            }
            std::process::exit(-1);
        }
    }
}

/// Sends an apropreate error/warning message for a `SettingsValidationResult`
fn handle_setting_validation(res: SettingsValidationResult, state_context: &StateContext) {
    match res {
        SettingsValidationResult::NoIssue => {}
        SettingsValidationResult::Warning(slicer_warning) => match state_context.display_type {
            DisplayType::Message => send_warning_message(slicer_warning),
            DisplayType::StdOut => show_warning_message(&slicer_warning),
        },
        SettingsValidationResult::Error(slicer_error) => {
            match state_context.display_type {
                DisplayType::Message => send_error_message(slicer_error),
                DisplayType::StdOut => show_error_message(&slicer_error),
            }
            std::process::exit(-1);
        }
    }
}
