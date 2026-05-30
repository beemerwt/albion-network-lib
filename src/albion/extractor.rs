use crate::albion::{
    ItemNameResolver, PlayerState, WorldMap
};

pub struct AlbionExtractor {
    player_state: PlayerState,
    
    item_names: ItemNameResolver,
    world_map: WorldMap,
}