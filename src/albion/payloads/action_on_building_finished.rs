use crate::{
    packet::RawParameters,
    util::{dotnet_ticks_to_unix_millis, value_i64},
};
use chrono::Utc;
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
            timestamp: value_i64(parameters, 1)
                .and_then(|ts| Some(dotnet_ticks_to_unix_millis(ts)))
                .unwrap_or_else(|| Utc::now().timestamp_millis()),
            building_id: value_i64(parameters, 2)?,
        })
    }
}
