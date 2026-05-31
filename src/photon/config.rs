use crate::albion::{ItemNameResolver, WorldMap};

pub struct PhotonParserConfig {
    pub source_name: String,
    pub debug: bool,
    pub capture_unknown_packets: bool,
    pub world_map: WorldMap,
    pub item_names: ItemNameResolver,
}

impl PhotonParserConfig {
    pub fn new(
        source_name: String,
        debug: bool,
        capture_unknown_packets: bool,
        world_map: WorldMap,
        item_names: ItemNameResolver,
    ) -> Self {
        Self {
            source_name,
            debug,
            capture_unknown_packets,
            world_map,
            item_names,
        }
    }

    pub fn with_defaults(source_name: String, debug: bool) -> Self {
        Self {
            source_name,
            debug,
            capture_unknown_packets: debug,
            world_map: WorldMap::from_embedded().unwrap_or_else(|_| WorldMap::empty()),
            item_names: ItemNameResolver::download_default().unwrap_or_else(|_| ItemNameResolver::empty()),
        }
    }
}
