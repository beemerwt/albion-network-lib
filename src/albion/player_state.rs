use crate::albion::AlbionLocation;

#[derive(Clone, Debug)]
pub struct PlayerState {
    location: AlbionLocation,
    player_name: String,
    albion_server: Option<String>,
    user_object_id: Option<i32>,
    has_encrypted_data: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            location: AlbionLocation::unknown(),
            player_name: String::new(),
            albion_server: None,
            user_object_id: None,
            has_encrypted_data: false,
        }
    }
}

impl PlayerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn location(&self) -> &AlbionLocation {
        &self.location
    }

    pub fn location_id(&self) -> Option<i64> {
        self.location.location_id()
    }

    pub fn location_index(&self) -> Option<&str> {
        self.location.location_index()
    }

    pub fn player_name(&self) -> &str {
        &self.player_name
    }

    pub fn has_encrypted_data(&self) -> bool {
        self.has_encrypted_data
    }

    pub fn user_object_id(&self) -> Option<i32> {
        self.user_object_id
    }

    pub fn set_location(&mut self, location: AlbionLocation) {
        self.location = location;
    }

    pub fn set_player_name(&mut self, player_name: impl Into<String>) {
        self.player_name = player_name.into();
    }

    pub fn set_albion_server(&mut self, albion_server: Option<String>) {
        self.albion_server = albion_server;
    }

    pub fn set_user_object_id(&mut self, user_object_id: Option<i32>) {
        self.user_object_id = user_object_id;
    }

    pub fn set_has_encrypted_data(&mut self, has_encrypted_data: bool) {
        self.has_encrypted_data = has_encrypted_data;
    }

    pub fn mark_encrypted_data_seen(&mut self) {
        self.has_encrypted_data = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::albion::AlbionLocation;

    #[test]
    fn setters_update_identity_and_flags() {
        let mut state = PlayerState::new();

        state.set_user_object_id(Some(42));
        state.set_player_name("TestPlayer");
        state.mark_encrypted_data_seen();

        assert_eq!(state.user_object_id(), Some(42));
        assert_eq!(state.player_name(), "TestPlayer");
        assert!(state.has_encrypted_data());
    }

    #[test]
    fn set_location_stores_resolved_location() {
        let mut state = PlayerState::new();

        state.set_location(AlbionLocation::with_names(
            "2000",
            "Bridgewatch",
            "Bridgewatch",
        ));

        assert_eq!(state.location().friendly_name(), "Bridgewatch");
        assert_eq!(state.location_id(), Some(2000));
    }
}
