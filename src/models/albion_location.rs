use serde::Serialize;

pub const UNKNOWN_LOCATION_ID: &str = "-2";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AlbionLocation {
    pub id: String,
    pub location_name: Option<String>,
    pub friendly_location_name: Option<String>,
}

impl AlbionLocation {
    pub fn unknown() -> Self {
        Self {
            id: UNKNOWN_LOCATION_ID.to_string(),
            location_name: None,
            friendly_location_name: None,
        }
    }

    pub fn from_id(id: impl Into<String>) -> Self {
        let id = id.into();
        if id.is_empty() {
            return Self::unknown();
        }
        Self {
            id,
            location_name: None,
            friendly_location_name: None,
        }
    }

    pub fn with_names(
        id: impl Into<String>,
        location_name: impl Into<String>,
        friendly_location_name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            location_name: Some(location_name.into()),
            friendly_location_name: Some(friendly_location_name.into()),
        }
    }

    pub fn is_unknown(&self) -> bool {
        self.id == UNKNOWN_LOCATION_ID
    }

    pub fn friendly_name(&self) -> &str {
        self.friendly_location_name
            .as_deref()
            .or(self.location_name.as_deref())
            .unwrap_or("Unknown")
    }

    pub fn location_id(&self) -> Option<i64> {
        if self.is_unknown() {
            return None;
        }
        self.id.parse().ok()
    }

    pub fn location_index(&self) -> Option<&str> {
        (!self.is_unknown()).then_some(self.id.as_str())
    }
}
