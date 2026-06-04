use crate::{packet::RawParameters, util::value_i64};
use serde::Serialize;

// Operation code 55
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActionOnBuildingStart {
    pub timestamp: i64,
    pub building_id: i64,
}

impl ActionOnBuildingStart {
    pub fn from_params(parameters: &RawParameters) -> Option<Self> {
        Some(Self {
            timestamp: value_i64(parameters, 0)?,
            building_id: value_i64(parameters, 1)?,
        })
    }
}
