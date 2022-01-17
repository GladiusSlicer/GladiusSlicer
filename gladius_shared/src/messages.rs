use crate::types::CalculatedValues;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug,Clone)]
pub enum Message{
    CalculatedValues(CalculatedValues)
}