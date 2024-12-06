use crate::{Command, Settings};
use evalexpr::{context_map, eval_float_with_context, DefaultNumericTypes, HashMapContext};
use gladius_shared::{error::SlicerErrors, settings::SettingsPrint, types::RetractionType};
use std::io::{BufWriter, Write};
use time::format_description::well_known::Iso8601;

pub fn convert(
    cmds: &[Command],
    settings: &Settings,
    write: &mut impl Write,
) -> Result<(), SlicerErrors> {
    let mut current_z = 0.0;
    let mut layer_count = 0;
    let mut current_object = None;
    let mut write_buf = BufWriter::new(write);

    // Add the slicing date to the g-code
    writeln!(
        write_buf,
        ";========== date: {} ==================",
        time::OffsetDateTime::now_utc()
            .to_offset(time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC))
            .format(&Iso8601::DATE)
            .expect("This is a valid format")
    )
    .map_err(|_| SlicerErrors::FileWriteError)?;

    // output the settings to the gcode file
    for line in settings.to_strings() {
        writeln!(
            //lending ; to make comment
            write_buf,
            "; {}",
            line
        )
        .map_err(|_| SlicerErrors::FileWriteError)?;
    }

    let start = convert_instructions(
        &settings.starting_instructions,
        current_z,
        layer_count,
        None,
        current_object,
        settings,
    )?;

    writeln!(
        write_buf,
        "M201 X{:.1} Y{:.1} Z{:.1} E{:.1}; sets maximum accelerations, mm/sec^2",
        settings.max_acceleration_x,
        settings.max_acceleration_y,
        settings.max_acceleration_z,
        settings.max_acceleration_e
    )
    .map_err(|_| SlicerErrors::FileWriteError)?;
    writeln!(
        write_buf,
        "M203 X{:.1} Y{:.1} Z{:.1} E{:.1}; ; sets maximum feedrates, mm/sec",
        settings.maximum_feedrate_x,
        settings.maximum_feedrate_y,
        settings.maximum_feedrate_z,
        settings.maximum_feedrate_e
    )
    .map_err(|_| SlicerErrors::FileWriteError)?;
    writeln!(
        write_buf,
        "M204 P{:.1} R{:.1} T{:.1}; sets acceleration (P, T) and retract acceleration (R), mm/sec^2",
        settings.max_acceleration_extruding,
        settings.max_acceleration_retracting,
        settings.max_acceleration_travel
    )
    .map_err(|_| SlicerErrors::FileWriteError)?;
    writeln!(
        write_buf,
        "M205 X{:.1} Y{:.1} Z{:.1} E{:.1}; sets the jerk limits, mm/sec",
        settings.max_jerk_x, settings.max_jerk_y, settings.max_jerk_z, settings.max_jerk_e
    )
    .map_err(|_| SlicerErrors::FileWriteError)?;
    writeln!(
        write_buf,
        "M205 S{:.1} T{:.1} ; sets the minimum extruding and travel feed rate, mm/sec",
        settings.minimum_feedrate_print, settings.minimum_feedrate_travel
    )
    .map_err(|_| SlicerErrors::FileWriteError)?;
    writeln!(write_buf, "{}", start).map_err(|_| SlicerErrors::FileWriteError)?;
    writeln!(write_buf, "G21 ; set units to millimeters")
        .map_err(|_| SlicerErrors::FileWriteError)?;
    writeln!(write_buf, "G90 ; use absolute Coords").map_err(|_| SlicerErrors::FileWriteError)?;
    writeln!(write_buf, "M83 ; use relative distances for extrusion")
        .map_err(|_| SlicerErrors::FileWriteError)?;

    for cmd in cmds {
        match cmd {
            Command::MoveTo { end, .. } => writeln!(write_buf, "G1 X{:.5} Y{:.5}", end.x, end.y)
                .map_err(|_| SlicerErrors::FileWriteError)?,
            Command::MoveAndExtrude {
                start,
                end,
                width,
                thickness,
            } => {
                let x_diff = end.x - start.x;
                let y_diff = end.y - start.y;
                let length = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();

                // let extrusion_width = width + (thickness * (1.0 - std::f64::consts::FRAC_PI_4));

                let extrusion_volume = (((width - thickness) * thickness)
                    + (std::f64::consts::PI * (thickness / 2.0) * (thickness / 2.0)))
                    * length;
                /*let extrusion_volume = width*thickness*length;*/

                let filament_area = (std::f64::consts::PI
                    * settings.filament.diameter
                    * settings.filament.diameter)
                    / 4.0;
                let extrude = extrusion_volume / filament_area;

                writeln!(write_buf, "G1 X{:.5} Y{:.5} E{:.5}", end.x, end.y, extrude)
                    .map_err(|_| SlicerErrors::FileWriteError)?;
            }
            Command::SetState { new_state } => {
                match &new_state.retract {
                    RetractionType::NoRetract => {
                        if let Some(speed) = new_state.movement_speed {
                            writeln!(write_buf, "G1 F{:.5}", speed * 60.0)
                                .map_err(|_| SlicerErrors::FileWriteError)?;
                        }
                        if let Some(accel) = new_state.acceleration {
                            writeln!(write_buf, "M204 S{:.1}", accel)
                                .map_err(|_| SlicerErrors::FileWriteError)?;
                        }
                    }
                    RetractionType::Retract => {
                        // retract
                        if let Some(speed) = new_state.movement_speed {
                            writeln!(write_buf, "G1 F{:.5}", speed * 60.0)
                                .map_err(|_| SlicerErrors::FileWriteError)?;
                        }
                        if let Some(accel) = new_state.acceleration {
                            writeln!(write_buf, "M204 S{:.1}", accel)
                                .map_err(|_| SlicerErrors::FileWriteError)?;
                        }

                        writeln!(
                            write_buf,
                            "G1 E{:.5} F{:.5}; Retract",
                            -settings.retract_length,
                            60.0 * settings.retract_speed,
                        )
                        .map_err(|_| SlicerErrors::FileWriteError)?;

                        writeln!(
                            write_buf,
                            "G1 Z{:.5} F{:.5}; z Lift",
                            current_z + settings.retract_lift_z,
                            60.0 * settings.speed.travel,
                        )
                        .map_err(|_| SlicerErrors::FileWriteError)?;
                    }
                    RetractionType::Unretract => {
                        // unretract
                        writeln!(write_buf, "G1 Z{:.5}; z unlift", current_z,)
                            .map_err(|_| SlicerErrors::FileWriteError)?;
                        writeln!(
                            write_buf,
                            "G1 E{:.5} F{:.5}; Unretract",
                            settings.retract_length,
                            60.0 * settings.retract_speed,
                        )
                        .map_err(|_| SlicerErrors::FileWriteError)?;

                        if let Some(speed) = new_state.movement_speed {
                            writeln!(write_buf, "G1 F{:.5}", speed * 60.0)
                                .map_err(|_| SlicerErrors::FileWriteError)?;
                        }
                        if let Some(accel) = new_state.acceleration {
                            writeln!(write_buf, "M204 S{:.1}", accel)
                                .map_err(|_| SlicerErrors::FileWriteError)?;
                        }
                    }
                    RetractionType::MoveRetract(moves) => {
                        if let Some(speed) = new_state.movement_speed {
                            writeln!(write_buf, "G1 F{:.5}", speed * 60.0)
                                .map_err(|_| SlicerErrors::FileWriteError)?;
                        }
                        if let Some(accel) = new_state.acceleration {
                            writeln!(write_buf, "M204 S{:.1}", accel)
                                .map_err(|_| SlicerErrors::FileWriteError)?;
                        }

                        for (retract_amount, end) in moves {
                            writeln!(
                                write_buf,
                                "G1 X{:.5} Y{:.5} E{:.5}; Retract with move",
                                end.x, end.y, -retract_amount
                            )
                            .map_err(|_| SlicerErrors::FileWriteError)?;
                        }

                        writeln!(
                            write_buf,
                            "G1 Z{:.5} F{:.5}; z Lift",
                            current_z + settings.retract_lift_z,
                            60.0 * settings.speed.travel,
                        )
                        .map_err(|_| SlicerErrors::FileWriteError)?;
                    }
                }

                if let Some(ext_temp) = new_state.extruder_temp {
                    writeln!(write_buf, "M104 S{:.1} ; set extruder temp", ext_temp)
                        .map_err(|_| SlicerErrors::FileWriteError)?;
                }
                if let Some(bed_temp) = new_state.bed_temp {
                    writeln!(write_buf, "M140 S{:.1} ; set bed temp", bed_temp)
                        .map_err(|_| SlicerErrors::FileWriteError)?;
                }
                if let Some(fan_speed) = new_state.fan_speed {
                    writeln!(
                        write_buf,
                        "M106 S{} ; set fan speed",
                        (2.550 * fan_speed).round() as usize
                    )
                    .map_err(|_| SlicerErrors::FileWriteError)?;
                }
                if settings.has_aux_fan {
                    if let Some(aux_fan_speed) = new_state.aux_fan_speed {
                        writeln!(
                            write_buf,
                            "M106 P2 S{} ; set aux fan speed",
                            (2.550 * aux_fan_speed).round() as usize
                        )
                        .map_err(|_| SlicerErrors::FileWriteError)?;
                    }
                }
            }
            Command::LayerChange { z, index } => {
                writeln!(
                    write_buf,
                    "{}",
                    convert_instructions(
                        &settings.before_layer_change_instructions,
                        current_z,
                        layer_count,
                        None,
                        current_object,
                        settings
                    )?
                )
                .map_err(|_| SlicerErrors::FileWriteError)
                .map_err(|_| SlicerErrors::FileWriteError)?;
                current_z = *z;
                layer_count = *index as u32;
                writeln!(write_buf, "G1 Z{:.5}", z)
                    .map_err(|_| SlicerErrors::FileWriteError)
                    .map_err(|_| SlicerErrors::FileWriteError)?;

                writeln!(
                    write_buf,
                    "{}",
                    convert_instructions(
                        &settings.after_layer_change_instructions,
                        current_z,
                        layer_count,
                        None,
                        current_object,
                        settings
                    )?
                )
                .map_err(|_| SlicerErrors::FileWriteError)?;
            }
            Command::Delay { msec } => {
                writeln!(write_buf, "G4 P{:.5}", msec).map_err(|_| SlicerErrors::FileWriteError)?;
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

                // Divide the chord length by double the radius.
                let t = cord_length / (2.0 * radius);

                // Find the inverse sine of the result (in radians).
                // Double the result of the inverse sine to get the central angle in radians.
                let central = t.asin() * 2.0;
                // Once you have the central angle in radians, multiply it by the radius to get the arc length.
                let extrusion_length = central * radius;

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
                    center.x - start.x,
                    center.y - start.y,
                    extrude
                )
                .map_err(|_| SlicerErrors::FileWriteError)?;
            }
            Command::ChangeObject { object } => {
                let previous_object = std::mem::replace(&mut current_object, Some(*object));
                writeln!(
                    write_buf,
                    "{}",
                    convert_instructions(
                        &settings.object_change_instructions,
                        current_z,
                        layer_count,
                        previous_object,
                        current_object,
                        settings
                    )?
                )
                .map_err(|_| SlicerErrors::FileWriteError)?;
            }
            Command::NoAction => {
                panic!("Converter reached a No Action Command, Optimization Failure")
            }
        }
    }

    let end = convert_instructions(
        &settings.ending_instructions,
        current_z,
        layer_count,
        None,
        current_object,
        settings,
    )?;

    writeln!(write_buf, "{end}").map_err(|_| SlicerErrors::FileWriteError)?;

    write_buf
        .flush()
        .expect("File Closed Before CLosed. Gcode invalid.");

    Ok(())
}

fn convert_instructions(
    instructions: &str,
    current_z_height: f64,
    layer_count: u32,
    previous_object: Option<usize>,
    current_object: Option<usize>,
    settings: &Settings,
) -> Result<String, SlicerErrors> {
    instructions
        .split('{')
        .enumerate()
        .map(|(index, str)| {
            // first one will not contain a }
            if index == 0 || str.is_empty() {
                Ok(String::from(str))
            } else {
                let mut split = str.split('}');
                let expression = split.next().ok_or(SlicerErrors::SettingMacroParseError {
                    sub_error: "Empty string".to_string(),
                })?;

                let mut response = parse_macro(
                    expression,
                    current_z_height,
                    layer_count,
                    previous_object,
                    current_object,
                    settings,
                )?;

                response += split.next().ok_or(SlicerErrors::SettingMacroParseError {
                    sub_error: "Missing end brace".to_string(),
                })?;

                Ok(response)
            }
        })
        .collect()
}

fn parse_macro(
    expression: &str,
    current_z_height: f64,
    layer_count: u32,
    previous_object: Option<usize>,
    current_object: Option<usize>,
    settings: &Settings,
) -> Result<String, SlicerErrors> {
    let layer_settings = settings.get_layer_settings(layer_count, current_z_height);

    let context: HashMapContext<DefaultNumericTypes> = context_map! {
        "curr_extruder_temp" => float layer_settings.extruder_temp,
        "current_extruder_temp" => float layer_settings.extruder_temp,
        "bed_temp" => float layer_settings.bed_temp,
        "z_pos" => float current_z_height,
        "layer_count" => float f64::from(layer_count),
        "prev_obj" => float previous_object.map_or(-1., |o| o  as f64),
        "curr_obj" => float current_object.map_or(-1., |o| o  as f64),
        "current_obj" => float current_object.map_or(-1., |o| o  as f64),
        "exterior_inner_perimeter_speed" => float layer_settings.speed.exterior_inner_perimeter,
        "exterior_surface_perimeter_speed" => float layer_settings.speed.exterior_surface_perimeter,
        "interior_inner_perimeter_speed" => float layer_settings.speed.interior_inner_perimeter,
        "interior_surface_perimeter_speed" => float layer_settings.speed.interior_surface_perimeter,
        "infill_speed" => float layer_settings.speed.infill,
        "solid_infill_speed" => float layer_settings.speed.solid_infill,
        "bridge_speed" => float layer_settings.speed.bridge,
        "travel_speed" => float layer_settings.speed.travel,
        "support_speed" => float layer_settings.speed.support,
        "print_size_x" => float settings.print_x,
        "print_size_y" => float settings.print_y,
        "print_size_z" => float settings.print_z,

    }
    .map_err(|e| SlicerErrors::SettingMacroParseError {
        sub_error: e.to_string(),
    })?;

    eval_float_with_context(expression, &context)
        .map_err(|e| SlicerErrors::SettingMacroParseError {
            sub_error: e.to_string(),
        })
        .map(|f| f.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_macro_test() {
        assert_eq!(
            parse_macro("1.5+3.0", 0.0, 0, Some(1), Some(2), &Settings::default()),
            Ok(String::from("4.5"))
        );
        assert_eq!(
            parse_macro(
                "curr_extruder_temp",
                0.0,
                0,
                Some(1),
                Some(2),
                &Settings::default()
            ),
            Ok(String::from("210"))
        );
    }

    #[test]
    fn convert_instructions_test() {
        assert_eq!(
            convert_instructions("{1.5+3.0}", 0.0, 0, Some(1), Some(2), &Settings::default()),
            Ok(String::from("4.5"))
        );
        assert_eq!(
            convert_instructions(
                "{curr_extruder_temp+3.0}",
                0.0,
                0,
                Some(1),
                Some(2),
                &Settings::default()
            ),
            Ok(String::from("213"))
        );
        assert_eq!(
            convert_instructions(
                "// temp is {curr_extruder_temp+3.0} C",
                0.0,
                0,
                Some(1),
                Some(2),
                &Settings::default()
            ),
            Ok(String::from("// temp is 213 C"))
        );
        assert_eq!(
            convert_instructions(
                "// temp is {if(10>20,10.0,20.0)} C",
                0.0,
                0,
                Some(1),
                Some(2),
                &Settings::default()
            ),
            Ok(String::from("// temp is 20 C"))
        );
    }
}
