#![deny(missing_docs)]

use crate::error::SlicerErrors;
use crate::types::{CalculatedValues, Command};
use crate::warning::SlicerWarnings;
use serde::{Deserialize, Serialize};

/// Messages for communicating between the slicer and another process
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    /// Message to share `CalculatedValues`` struct
    CalculatedValues(CalculatedValues),

    /// Message to share the list of all commands
    Commands(Vec<Command>),

    /// Message to share final Gcode
    GCode(String),

    /// Message to share the current state of the slicer
    StateUpdate(String),

    /// Message to share any errors encountered
    Error(SlicerErrors),

    /// Message to share any warnings encountered
    Warning(SlicerWarnings),
}
