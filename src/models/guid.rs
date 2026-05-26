use serde::Serialize;
use serde_json::Value;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl Guid {
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::String(value) => Self::from_str(value),
            Value::Object(map) => map
                .get("data_hex")
                .and_then(Value::as_str)
                .and_then(Self::from_photon_data_hex),
            _ => None,
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        let hex: String = value.chars().filter(|char| *char != '-').collect();
        if hex.len() != 32 || !hex.chars().all(|char| char.is_ascii_hexdigit()) {
            return None;
        }

        Some(Self {
            data1: u32::from_str_radix(&hex[0..8], 16).ok()?,
            data2: u16::from_str_radix(&hex[8..12], 16).ok()?,
            data3: u16::from_str_radix(&hex[12..16], 16).ok()?,
            data4: [
                u8::from_str_radix(&hex[16..18], 16).ok()?,
                u8::from_str_radix(&hex[18..20], 16).ok()?,
                u8::from_str_radix(&hex[20..22], 16).ok()?,
                u8::from_str_radix(&hex[22..24], 16).ok()?,
                u8::from_str_radix(&hex[24..26], 16).ok()?,
                u8::from_str_radix(&hex[26..28], 16).ok()?,
                u8::from_str_radix(&hex[28..30], 16).ok()?,
                u8::from_str_radix(&hex[30..32], 16).ok()?,
            ],
        })
    }

    pub fn from_photon_data_hex(value: &str) -> Option<Self> {
        let hex: String = value
            .chars()
            .filter(|char| !char.is_ascii_whitespace() && *char != '-')
            .collect();
        if hex.len() != 32 || !hex.chars().all(|char| char.is_ascii_hexdigit()) {
            return None;
        }

        let mut bytes = [0; 16];
        for index in 0..16 {
            bytes[index] = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
        }

        Some(Self {
            data1: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            data2: u16::from_le_bytes(bytes[4..6].try_into().ok()?),
            data3: u16::from_le_bytes(bytes[6..8].try_into().ok()?),
            data4: bytes[8..16].try_into().ok()?,
        })
    }
}

impl Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let guid_string = format!(
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7]
        );
        serializer.serialize_str(&guid_string)
    }
}
