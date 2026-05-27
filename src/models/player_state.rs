use crate::models::{AlbionLocation, WorldMap};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct PlayerState {
    world_map: Arc<WorldMap>,
    pub location: AlbionLocation,
    pub player_name: String,
    pub albion_server: Option<String>,
    pub user_object_id: Option<i32>,
    pub has_encrypted_data: bool,
}

impl PlayerState {
    pub fn new(world_map: Arc<WorldMap>) -> Self {
        Self {
            world_map,
            location: AlbionLocation::unknown(),
            player_name: String::new(),
            albion_server: None,
            user_object_id: None,
            has_encrypted_data: false,
        }
    }

    pub fn location_id(&self) -> Option<i64> {
        self.location.location_id()
    }

    pub fn location_index(&self) -> Option<&str> {
        self.location.location_index()
    }

    pub fn user_object_id(&self) -> Option<i32> {
        self.user_object_id
    }

    pub fn set_has_encrypted_data(&mut self, has_encrypted_data: bool) {
        self.has_encrypted_data = has_encrypted_data;
    }

    pub fn set_location_raw(&mut self, location: &str) {
        self.location = self.world_map.resolve_location(location);
    }

    pub fn set_player_name(&mut self, player_name: impl Into<String>) {
        let player_name = player_name.into();
        if self.player_name != player_name {
            self.player_name = player_name;
        }
    }

    pub fn set_albion_server(&mut self, albion_server: Option<String>) {
        if self.albion_server != albion_server {
            self.albion_server = albion_server;
        }
    }

    pub fn set_user_object_id(&mut self, user_object_id: Option<i32>) {
        self.user_object_id = user_object_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> PlayerState {
        PlayerState::new(Arc::new(WorldMap::from_embedded().unwrap()))
    }

    #[test]
    fn defaults_match_csharp_intent() {
        let state = state();

        assert_eq!(state.location, AlbionLocation::unknown());
        assert_eq!(state.player_name, "");
        assert_eq!(state.user_object_id(), None);
    }

    #[test]
    fn join_style_updates_set_identity_and_location() {
        let mut state = state();

        state.set_user_object_id(Some(42));
        state.set_player_name("TestPlayer");
        state.set_location_raw("Bridgewatch");

        assert_eq!(state.user_object_id(), Some(42));
        assert_eq!(state.player_name, "TestPlayer");
        assert_eq!(state.location.friendly_name(), "Bridgewatch");
        assert_eq!(state.location.location_id(), Some(2000));
    }
}
