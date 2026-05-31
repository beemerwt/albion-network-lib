use crate::albion::{ItemNameResolver, WorldMap};

pub struct PhotonParserConfig {
    pub source_name: String,
    pub debug: bool,
    pub capture_unknown_packets: bool,
    pub world_map: WorldMap,
    pub item_names: ItemNameResolver,
}
