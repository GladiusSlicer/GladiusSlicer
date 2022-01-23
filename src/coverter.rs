use crate::{Command, Settings};
use std::io::{BufWriter, Write};

pub fn convert(
    cmds: &[Command],
    settings: Settings,
    write: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_z = 0.0;

    let mut start = settings.starting_instructions.clone();
    let mut write_buf = BufWriter::new(write);
    let layer_settings = settings.get_layer_settings(0, 0.0);

    start = start.replace(
        "[First Layer Extruder Temp]",
        &format!("{:.1}", layer_settings.extruder_temp),
    );
    start = start.replace(
        "[First Layer Bed Temp]",
        &format!("{:.1}", layer_settings.bed_temp),
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
                /*let extrusion_volume = width*thickness*length;*/

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
                            60.0 * settings.speed.travel,
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
                if let Some(accel) = new_state.acceleration {
                    writeln!(write_buf, "M204 S{:.1}", accel)?;
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
                writeln!(write_buf, "G92 E0.0")?;
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

    let end = settings.ending_instructions;

    writeln!(write_buf, "{}", end)?;

    write_buf
        .flush()
        .expect("File Closed Before CLosed. Gcode invalid.");

    Ok(())
}
