use crate::{packet::RawParameters, util::value_i64};
use serde::Serialize;

// Event code 50
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepairBuildingInfo {
    pub building_id: i64,
}

impl RepairBuildingInfo {
    pub fn from_params(parameters: &RawParameters) -> Option<Self> {
        Some(Self {
            building_id: value_i64(parameters, 0)?
        })
    }
}
