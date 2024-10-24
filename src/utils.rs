use geo::Coord;
use gladius_shared::error::SlicerErrors;
use gladius_shared::messages::Message;
use gladius_shared::warning::SlicerWarnings;
use log::{error, info, warn};
use nalgebra::Vector2;
use std::io::Write;

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
    let stdout = std::io::stdout();
    let mut stdio_lock = stdout.lock();

    let message = Message::Error(error);
    bincode::serialize_into(&mut stdio_lock, &message).expect("Write Limit should not be hit");
    stdio_lock.flush().expect("Standard Out should be limited");
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
    let stdout = std::io::stdout();
    let mut stdio_lock = stdout.lock();
    let message = Message::Warning(warning);
    bincode::serialize_into(&mut stdio_lock, &message).expect("Write Limit should not be hit");
    stdio_lock.flush().expect("Standard Out should be limited");
}

pub fn display_state_update(state_message: &str, send_message: bool) {
    if send_message {
        let stdout = std::io::stdout();
        let mut stdio_lock = stdout.lock();
        let message = Message::StateUpdate(state_message.to_string());
        bincode::serialize_into(&mut stdio_lock, &message).expect("Write Limit should not be hit");
        stdio_lock.flush().expect("Standard Out should be limited");
    } else {
        info!("{}", state_message);
    }
}

#[inline]
pub fn point_y_lerp(a: &Coord<f64>, b: &Coord<f64>, y: f64) -> Coord<f64> {
    Coord {
        x: lerp(a.x, b.x, (y - a.y) / (b.y - a.y)),
        y,
    }
}

#[inline]
pub fn point_lerp(a: &Coord<f64>, b: &Coord<f64>, f: f64) -> Coord<f64> {
    Coord {
        x: lerp(a.x, b.x, f),
        y: lerp(a.y, b.y, f),
    }
}

#[inline]
pub fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + f * (b - a)
}

/// Function to generate a unit bisector of the angle p0, p1, p2 that will always be inside the angle to the left
pub fn directional_unit_bisector_left(
    p0: &Coord<f64>,
    p1: &Coord<f64>,
    p2: &Coord<f64>,
) -> Vector2<f64> {
    let v1 = Vector2::new(p0.x - p1.x, p0.y - p1.y);
    let v2 = Vector2::new(p2.x - p1.x, p2.y - p1.y);

    let v1_scale = v1 * v2.magnitude();
    let v2_scale = v2 * v1.magnitude();

    let direction = v1_scale + v2_scale;

    match orientation(p0, p1, p2) {
        Orientation::Linear => {
            let perp = Vector2::new(-v1.y, v1.x).normalize();
            match orientation(p0, p1, &Coord::from((p1.x + perp.x, p1.y + perp.y))) {
                Orientation::Linear => {
                    unreachable!()
                }
                Orientation::Left => perp.normalize(),
                Orientation::Right => perp.normalize().scale(-1.0),
            }
        }
        Orientation::Left => direction.normalize(),
        Orientation::Right => direction.normalize().scale(-1.0),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Orientation {
    Linear,
    Left,
    Right,
}

pub fn orientation(p: &Coord<f64>, q: &Coord<f64>, r: &Coord<f64>) -> Orientation {
    let left_val = (q.x - p.x) * (r.y - p.y);
    let right_val = (q.y - p.y) * (r.x - p.x);

    if left_val == right_val {
        Orientation::Linear
    } else if left_val > right_val {
        Orientation::Left
    } else {
        Orientation::Right
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_directional_unit_bisector() {
        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((0.0, 0.0)),
                &Coord::from((1.0, 0.0)),
                &Coord::from((1.0, 1.0))
            ),
            Vector2::new(-1.0, 1.0).normalize()
        );
        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((1.0, 1.0)),
                &Coord::from((1.0, 0.0)),
                &Coord::from((0.0, 0.0))
            ),
            Vector2::new(1.0, -1.0).normalize()
        );

        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((0.0, 0.0)),
                &Coord::from((1.0, 0.0)),
                &Coord::from((2.0, 0.0))
            ),
            Vector2::new(0.0, 1.0)
        );
        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((2.0, 0.0)),
                &Coord::from((1.0, 0.0)),
                &Coord::from((0.0, 0.0))
            ),
            Vector2::new(0.0, -1.0)
        );

        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((0.0, 0.0)),
                &Coord::from((0.0, 1.0)),
                &Coord::from((0.0, 1.0))
            ),
            Vector2::new(-1.0, 0.0)
        );
        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((0.0, 2.0)),
                &Coord::from((0.0, 1.0)),
                &Coord::from((0.0, 0.0))
            ),
            Vector2::new(1.0, 0.0)
        );
    }
}
