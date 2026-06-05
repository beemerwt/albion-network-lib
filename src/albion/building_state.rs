use chrono::Utc;
use serde::Serialize;

use crate::{
    albion::payloads::{ActionOnBuildingFinished, ActionOnBuildingStart, RepairBuildingInfo},
    packet::RawParameters,
    util::{dotnet_ticks_to_unix_millis, i64_array, value_i32, value_i64},
};

#[derive(Clone, Debug, Default, Serialize)]
pub struct BuildingState {
    pub active_building: Option<i64>,
    pub active_action: Option<BuildingAction>,
    pub repair_building_info: Option<RepairBuildingInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
enum BuildingActionKind {
    Repair,
}

impl BuildingActionKind {
    pub fn from_params(self, parameters: &RawParameters) -> Option<BuildingAction> {
        match self {
            BuildingActionKind::Repair => BuildingAction::from_repair_params(parameters),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BuildingAction {
    Repair(Repair),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Repair {
    pub building_id: i64,
    pub started_at: i64,
    pub finished_at: i64,
    pub num_items: i32,
    pub cost: i32,
    pub item_ids: Vec<i64>,
}

impl BuildingAction {
    fn from_repair_params(parameters: &RawParameters) -> Option<Self> {
        Some(Self::Repair(Repair {
            started_at: value_i64(parameters, 0)
                .and_then(|ts| Some(dotnet_ticks_to_unix_millis(ts)))
                .unwrap_or_else(|| Utc::now().timestamp_millis()),
            building_id: value_i64(parameters, 1)?,
            num_items: value_i32(parameters, 2)?,
            cost: value_i32(parameters, 4)?,
            item_ids: i64_array(parameters, 5),
            finished_at: 0,
        }))
    }
}

impl BuildingState {
    pub fn new() -> Self {
        Self {
            active_building: None,
            active_action: None,
            repair_building_info: None,
        }
    }

    pub fn mark_repair_building(&mut self, info: RepairBuildingInfo) {
        self.repair_building_info = Some(info);
    }

    pub fn begin_action(&mut self, parameters: &RawParameters) {
        let Some(start) = ActionOnBuildingStart::from_params(parameters) else {
            println!(
                "Failed to parse ActionOnBuildingStart from parameters: {:?}",
                parameters
            );
            return;
        };

        println!("Starting action on building with ID: {}", start.building_id);

        self.active_building = Some(start.building_id);

        let pending = self.active_building.and_then(|id| {
            let info = self.repair_building_info.as_ref()?;
            match id {
                _ if id == info.building_id => BuildingActionKind::Repair.from_params(parameters),
                _ => None,
            }
        });

        self.active_action = pending;
    }

    pub fn cancel_action(&mut self) {
        self.active_action = None;
    }

    pub fn finish_action(&self, parameters: &RawParameters) -> Option<BuildingAction> {
        let Some(finish) = ActionOnBuildingFinished::from_params(parameters) else {
            println!(
                "Failed to parse ActionOnBuildingFinished from parameters: {:?}",
                parameters
            );
            return None;
        };

        self.active_building.and_then(|id| {
            if let Some(info) = &self.repair_building_info {
                if id == info.building_id && finish.building_id == info.building_id {
                    println!(
                        "Returning repair action for building ID: {}",
                        info.building_id
                    );
                    return self.active_action.clone();
                }
            }
            None
        })
    }
}
