use crate::{packet::RawParameters, util::value_i64};
use serde::Serialize;

// Operation code 56, event code 65
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActionOnBuildingCancel {
    pub timestamp: i64,
    pub building_id: i64,
}

impl ActionOnBuildingCancel {
    pub fn from_params(parameters: &RawParameters) -> Option<Self> {
        Some(Self {
            timestamp: value_i64(parameters, 1)?,
            building_id: value_i64(parameters, 2)?,
        })
    }
}
