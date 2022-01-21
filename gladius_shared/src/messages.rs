use crate::types::{CalculatedValues, Command};
use serde::{Deserialize, Serialize};
use crate::error::SlicerErrors;


#[derive(Serialize, Deserialize, Debug,Clone)]
pub enum Message{
    CalculatedValues(CalculatedValues),
    Commands(Vec<Command>),
    GCode(String),
    Error(SlicerErrors)
}