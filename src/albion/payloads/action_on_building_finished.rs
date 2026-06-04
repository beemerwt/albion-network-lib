use crate::{ packet::RawParameters, util::value_i64 };
use serde::Serialize;

// Event code 66
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActionOnBuildingFinished {
    pub timestamp: i64,
    pub building_id: i64,
}

impl ActionOnBuildingFinished {
    pub fn from_params(parameters: &RawParameters) -> Option<Self> {
        Some(Self {
            timestamp: value_i64(parameters, 1)?,
            building_id: value_i64(parameters, 2)?,
        })
    }
}
