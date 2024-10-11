use gladius_shared::error::SlicerErrors;
use gladius_shared::settings::Settings;
use gladius_shared::types::{Command, IndexedTriangle, Vertex};
use itertools::Itertools;

pub fn check_model_bounds(
    models: &[(Vec<Vertex>, Vec<IndexedTriangle>)],
    settings: &Settings,
) -> Result<(), SlicerErrors> {
    let brim_width = settings.brim_width.unwrap_or(0.0);
    let shrink_distance = settings.layer_shrink_amount.unwrap_or(0.0);

    let total_offset = brim_width + shrink_distance;

    models
        .iter()
        .flat_map(|model| model.0.iter())
        .map(|v| {
            if v.x < total_offset
                || v.y < total_offset
                || v.z < -0.00001
                || v.x > settings.print_x - total_offset
                || v.y > settings.print_y - total_offset
                || v.z > settings.print_z
            {
                Err(SlicerErrors::ModelOutsideBuildArea)
            } else {
                Ok(())
            }
        })
        .try_collect()
}
pub fn check_moves_bounds(moves: &[Command], settings: &Settings) -> Result<(), SlicerErrors> {
    moves
        .iter()
        .map(|command| match command {
            Command::MoveTo { end, .. } | Command::MoveAndExtrude { end, .. } => {
                if end.x < 0.0
                    || end.x > settings.print_x
                    || end.y < 0.0
                    || end.y > settings.print_y
                {
                    Err(SlicerErrors::MovesOutsideBuildArea)
                } else {
                    Ok(())
                }
            }
            Command::LayerChange { z, .. } => {
                if *z > settings.print_z || *z < 0.0 {
                    Err(SlicerErrors::MovesOutsideBuildArea)
                } else {
                    Ok(())
                }
            }
            Command::Arc { .. } => {
                unimplemented!()
            }
            Command::SetState { .. }
            | Command::Delay { .. }
            | Command::NoAction
            | Command::ChangeObject { .. } => Ok(()),
        })
        .try_collect()
}
