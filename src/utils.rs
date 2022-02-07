use geo::Coordinate;
use gladius_shared::error::SlicerErrors;
use gladius_shared::messages::Message;
use gladius_shared::warning::SlicerWarnings;
use log::{error, info, warn};
use std::io::{BufWriter, Write};

pub fn show_error_message(error: SlicerErrors) {
    let (error_code, message) = error.get_code_and_message();
    error!("\n");
    error!("**************************************************");
    error!("\tGladius Slicer Ran into an error");
    error!("\tError Code: {:#X}", error_code);
    error!("\t{}", message);
    error!("**************************************************");
    error!("\n\n\n");
}
pub fn send_error_message(error: SlicerErrors) {
    let message = Message::Error(error);
    bincode::serialize_into(BufWriter::new(std::io::stdout()), &message).unwrap();
    std::io::stdout()
        .flush()
        .expect("Standard Out should be limited");
}

pub fn show_warning_message(warning: SlicerWarnings) {
    let (error_code, message) = warning.get_code_and_message();
    warn!("\n");
    warn!("**************************************************");
    warn!("\tGladius Slicer found a warning");
    warn!("\tWarning Code: {:#X}", error_code);
    warn!("\t{}", message);
    warn!("**************************************************");
    warn!("\n\n\n");
}
pub fn send_warning_message(warning: SlicerWarnings) {
    let message = Message::Warning(warning);
    bincode::serialize_into(BufWriter::new(std::io::stdout()), &message).unwrap();
    std::io::stdout()
        .flush()
        .expect("Standard Out should be limited");
}

pub fn display_state_update(state_message: &str, send_message: bool) {
    if send_message {
        let message = Message::StateUpdate(state_message.to_string());
        bincode::serialize_into(std::io::stdout(), &message).unwrap();
        std::io::stdout()
            .flush()
            .expect("Standard Out should be limited");
    } else {
        info!("{}", state_message);
    }
}

#[inline]
pub fn point_y_lerp(a: &Coordinate<f64>, b: &Coordinate<f64>, y: f64) -> Coordinate<f64> {
    Coordinate {
        x: lerp(a.x, b.x, (y - a.y) / (b.y - a.y)),
        y,
    }
}

#[inline]
pub fn point_lerp(a: &Coordinate<f64>, b: &Coordinate<f64>, f: f64) -> Coordinate<f64> {
    Coordinate {
        x: lerp(a.x, b.x, f),
        y: lerp(a.y, b.y, f),
    }
}

#[inline]
pub fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + f * (b - a)
}
