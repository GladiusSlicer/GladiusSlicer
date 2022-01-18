use crate::types::CalculatedValues;
use serde::{Deserialize, Serialize};
use crate::error::SlicerErrors;


#[derive(Serialize, Deserialize, Debug,Clone)]
pub enum Message{
    CalculatedValues(CalculatedValues),
    Error(SlicerErrors)
}