use crate::models::AlbionLocation;
use serde::Deserialize;
use std::collections::HashMap;

const EMBEDDED_WORLD_JSON: &str = include_str!("../../world.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WorldEntry {
    index: String,
    unique_name: String,
}

#[derive(Clone, Debug)]
pub struct WorldMap {
    by_index: HashMap<String, String>,
    by_name: HashMap<String, String>,
}

impl WorldMap {
    pub fn empty() -> Self {
        Self {
            by_index: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    pub fn from_embedded() -> Result<Self, serde_json::Error> {
        Self::from_json_str(EMBEDDED_WORLD_JSON)
    }

    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        let entries: Vec<WorldEntry> = serde_json::from_str(json)?;

        let mut by_index = HashMap::new();
        let mut by_name = HashMap::new();

        for entry in entries {
            by_name.insert(entry.unique_name.clone(), entry.index.clone());
            by_index.insert(entry.index, entry.unique_name);
        }

        Ok(Self { by_index, by_name })
    }

    pub fn name_from_index(&self, index: &str) -> Option<&str> {
        self.by_index.get(index).map(String::as_str)
    }

    pub fn index_from_name(&self, name: &str) -> Option<&str> {
        self.by_name.get(name).map(String::as_str)
    }

    pub fn resolve_location(&self, value: &str) -> AlbionLocation {
        if value.eq_ignore_ascii_case("unset") {
            return AlbionLocation::Unset;
        }
        if value.eq_ignore_ascii_case("unknown") {
            return AlbionLocation::Unknown;
        }
        if let Some(unique_name) = self.name_from_index(value) {
            return AlbionLocation::Known {
                index: value.to_string(),
                unique_name: unique_name.to_string(),
            };
        }
        if let Some(index) = self.index_from_name(value) {
            return AlbionLocation::Known {
                index: index.to_string(),
                unique_name: value.to_string(),
            };
        }
        AlbionLocation::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::{AlbionLocation, WorldMap};

    #[test]
    fn loads_embedded_world_map() {
        let world_map = WorldMap::from_embedded().unwrap();

        assert_eq!(world_map.index_from_name("Bridgewatch"), Some("2000"));
        assert_eq!(world_map.name_from_index("2000"), Some("Bridgewatch"));
    }

    #[test]
    fn resolves_locations_by_index_and_unique_name() {
        let world_map = WorldMap::from_embedded().unwrap();

        assert_eq!(
            world_map.resolve_location("2000"),
            AlbionLocation::Known {
                index: "2000".to_string(),
                unique_name: "Bridgewatch".to_string(),
            }
        );
        assert_eq!(
            world_map.resolve_location("Bridgewatch"),
            AlbionLocation::Known {
                index: "2000".to_string(),
                unique_name: "Bridgewatch".to_string(),
            }
        );
    }

    #[test]
    fn location_id_only_uses_numeric_indexes() {
        let numeric = AlbionLocation::Known {
            index: "3008".to_string(),
            unique_name: "Bridgewatch".to_string(),
        };
        let non_numeric = AlbionLocation::Known {
            index: "BLACKBANK-2310".to_string(),
            unique_name: "Test".to_string(),
        };

        assert_eq!(numeric.location_id(), Some(3008));
        assert_eq!(non_numeric.location_id(), None);
        assert_eq!(AlbionLocation::Unknown.location_id(), None);
    }
}
