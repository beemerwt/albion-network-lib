use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum AlbionLocation {
    Unset,
    Unknown,
    Known { index: String, unique_name: String },
}

impl AlbionLocation {
    pub fn friendly_name(&self) -> &str {
        match self {
            Self::Unset => "Unset",
            Self::Unknown => "Unknown",
            Self::Known { unique_name, .. } => unique_name,
        }
    }

    pub fn location_id(&self) -> Option<i64> {
        match self {
            Self::Known { index, .. } => index.parse().ok(),
            Self::Unset | Self::Unknown => None,
        }
    }

    pub fn location_index(&self) -> Option<&str> {
        match self {
            Self::Known { index, .. } => Some(index),
            Self::Unset | Self::Unknown => None,
        }
    }
}
