use crate::*;

pub fn calculate_values(moves: &[Command], settings: &Settings) -> CalculatedValues {
    let mut values = CalculatedValues {
        plastic_volume: 0.0,
        plastic_weight: 0.0,
        total_time: 0.0,
        plastic_length: 0.0,
    };

    let mut current_speed = 0.0;
    let mut current_pos = Coordinate { x: 0.0, y: 0.0 };

    for cmd in moves {
        match cmd {
            Command::MoveTo { end } => {
                let x_diff = end.x - current_pos.x;
                let y_diff = end.y - current_pos.y;
                let d = ((x_diff * x_diff) + (y_diff * y_diff)).sqrt();
                current_pos = *end;
                if current_speed != 0.0 {
                    values.total_time += d / current_speed;
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
                values.total_time += d / current_speed;

                values.plastic_volume += width * thickness * d;
            }
            Command::SetState { new_state } => {
                if let Some(speed) = new_state.movement_speed {
                    current_speed = speed
                }
                if new_state.retract.is_some() {
                    values.total_time += settings.retract_length / settings.retract_speed;
                    values.total_time += settings.retract_lift_z / settings.speed.travel;
                }
            }
            Command::Delay { msec } => {
                values.total_time += *msec as f64 / 1000.0;
            }
            Command::Arc { .. } => {
                unimplemented!()
            }
            Command::NoAction | Command::LayerChange { .. } | Command::ChangeObject { .. } => {}
        }
    }

    values.plastic_weight = (values.plastic_volume / 1000.0) * settings.filament.density;
    values.plastic_length = values.plastic_volume
        / (std::f64::consts::PI
            * (settings.nozzle_diameter / 2.0)
            * (settings.nozzle_diameter / 2.0));

    values
}
