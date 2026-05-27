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
        if value.trim().is_empty()
            || value.eq_ignore_ascii_case("unset")
            || value.eq_ignore_ascii_case("unknown")
            || value == "-2"
        {
            return AlbionLocation::unknown();
        }
        if let Some(unique_name) = self.name_from_index(value) {
            return AlbionLocation::with_names(value, unique_name, unique_name);
        }
        if let Some(index) = self.index_from_name(value) {
            return AlbionLocation::with_names(index, value, value);
        }
        AlbionLocation::unknown()
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
            AlbionLocation::with_names("2000", "Bridgewatch", "Bridgewatch")
        );
        assert_eq!(
            world_map.resolve_location("Bridgewatch"),
            AlbionLocation::with_names("2000", "Bridgewatch", "Bridgewatch")
        );
    }

    #[test]
    fn resolves_non_numeric_indexes_and_unknowns() {
        let world_map = WorldMap::from_embedded().unwrap();

        assert_eq!(
            world_map.resolve_location("ISLAND-GUILD-0001a"),
            AlbionLocation::with_names(
                "ISLAND-GUILD-0001a",
                "ISLAND-GUILD-0001a_ISL_DL_T1_NON",
                "ISLAND-GUILD-0001a_ISL_DL_T1_NON",
            )
        );
        assert_eq!(
            world_map.resolve_location("UNKNOWN-NON-NUMERIC"),
            AlbionLocation::unknown()
        );
        assert_eq!(world_map.resolve_location("").id, "-2");
    }
}
