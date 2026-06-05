use crate::{error::Result, packet::RawParameters};
use serde_json::{Value, json};
use std::convert::TryFrom;

const DOTNET_EPOCH_TICKS: i64 = 621_355_968_000_000_000;
const TICKS_PER_MILLISECOND: i64 = 10_000;

// Raw parameter numeric extraction

fn json_value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Bool(value) => Some(i64::from(*value)),
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_u64().and_then(|value| i64::try_from(value).ok())),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

fn json_value_as<T>(value: &Value) -> Option<T>
where
    T: TryFrom<i64>,
{
    json_value_to_i64(value).and_then(|value| T::try_from(value).ok())
}

pub fn value_i32(params: &RawParameters, key: u8) -> Option<i32> {
    params
        .get(key)
        .and_then(json_value_to_i64)
        .map(|value| value as i32)
}

pub fn value_i64(params: &RawParameters, key: u8) -> Option<i64> {
    params.get(key).and_then(json_value_to_i64)
}

pub fn value_u8(params: &RawParameters, key: u8) -> Option<u8> {
    params.get(key).and_then(json_value_as)
}

// Raw parameter array extraction

pub fn i64_array(params: &RawParameters, key: u8) -> Vec<i64> {
    params
        .get(key)
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(json_value_to_i64).collect())
        .unwrap_or_default()
}

pub fn string_array(params: &RawParameters, key: u8) -> Vec<String> {
    params
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

// JSON/protocol helpers

pub fn to_signed_short(value: i64) -> i32 {
    let mut value = (value & 0xffff) as i32;
    if value >= 0x8000 {
        value -= 0x10000;
    }
    value
}

pub fn json_key(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

pub fn bytes_value(bytes: &[u8]) -> Value {
    json!({"bytes_hex": hex_lower(bytes)})
}

// Binary readers

fn read_array<const N: usize>(data: &[u8], offset: usize) -> Result<[u8; N]> {
    Ok(data
        .get(offset..offset + N)
        .ok_or("Unexpected end of data")?
        .try_into()
        .unwrap())
}

pub fn read_u16(data: &[u8], offset: usize, little: bool) -> Result<u16> {
    let bytes = read_array(data, offset)?;
    Ok(if little {
        u16::from_le_bytes(bytes)
    } else {
        u16::from_be_bytes(bytes)
    })
}

pub fn read_u32(data: &[u8], offset: usize, little: bool) -> Result<u32> {
    let bytes = read_array(data, offset)?;
    Ok(if little {
        u32::from_le_bytes(bytes)
    } else {
        u32::from_be_bytes(bytes)
    })
}

pub fn read_i32_be(data: &[u8], offset: usize) -> Result<i32> {
    Ok(i32::from_be_bytes(read_array(data, offset)?))
}

// Hex helpers

pub fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn hex_upper(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

// Time conversion

pub fn dotnet_ticks_to_unix_millis(ticks: i64) -> i64 {
    if ticks >= DOTNET_EPOCH_TICKS {
        (ticks - DOTNET_EPOCH_TICKS) / TICKS_PER_MILLISECOND
    } else {
        ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn params(values: &[(u8, Value)]) -> RawParameters {
        let mut params = RawParameters::empty();
        for (key, value) in values {
            params.insert(*key, value.clone());
        }
        params
    }

    #[test]
    fn value_i64_accepts_supported_json_numeric_shapes() {
        let params = params(&[
            (0, json!(42)),
            (1, json!("43")),
            (2, json!(true)),
            (3, json!(false)),
            (4, json!(u64::MAX)),
            (5, json!("nope")),
        ]);

        assert_eq!(value_i64(&params, 0), Some(42));
        assert_eq!(value_i64(&params, 1), Some(43));
        assert_eq!(value_i64(&params, 2), Some(1));
        assert_eq!(value_i64(&params, 3), Some(0));
        assert_eq!(value_i64(&params, 4), None);
        assert_eq!(value_i64(&params, 5), None);
        assert_eq!(value_i64(&params, 99), None);
    }

    #[test]
    fn checked_integer_helpers_reject_overflow_and_invalid_values() {
        let params = params(&[
            (0, json!(i32::MAX)),
            (1, json!(u32::MAX)),
            (2, json!(255)),
            (3, json!(256)),
            (4, json!(-1)),
            (5, json!("7")),
            (6, json!("bad")),
        ]);

        assert_eq!(value_i32(&params, 0), Some(i32::MAX));
        assert_eq!(value_i32(&params, 1), Some(-1));
        assert_eq!(value_i32(&params, 5), Some(7));
        assert_eq!(value_i32(&params, 6), None);
        assert_eq!(value_i32(&params, 99), None);

        assert_eq!(value_u8(&params, 2), Some(255));
        assert_eq!(value_u8(&params, 3), None);
        assert_eq!(value_u8(&params, 4), None);
        assert_eq!(value_u8(&params, 5), Some(7));
        assert_eq!(value_u8(&params, 99), None);
    }

    #[test]
    fn arrays_collect_supported_elements_and_ignore_malformed_items() {
        let params = params(&[
            (0, json!([1, "2", true, false, {"bad": true}, "bad"])),
            (1, json!(["Bridgewatch", 42, "BLACKBANK-2310", null])),
            (2, json!("not an array")),
        ]);

        assert_eq!(i64_array(&params, 0), vec![1, 2, 1, 0]);
        assert_eq!(
            string_array(&params, 1),
            vec!["Bridgewatch".to_string(), "BLACKBANK-2310".to_string()]
        );
        assert!(i64_array(&params, 2).is_empty());
        assert!(string_array(&params, 2).is_empty());
        assert!(i64_array(&params, 99).is_empty());
        assert!(string_array(&params, 99).is_empty());
    }

    #[test]
    fn signed_short_wraps_ushort_values() {
        assert_eq!(to_signed_short(0x0001), 1);
        assert_eq!(to_signed_short(0x7fff), 32767);
        assert_eq!(to_signed_short(0x8000), -32768);
        assert_eq!(to_signed_short(0xffff), -1);
        assert_eq!(to_signed_short(0x1_0001), 1);
    }

    #[test]
    fn json_and_hex_helpers_format_protocol_values() {
        assert_eq!(json_key(&json!("key")), "key");
        assert_eq!(json_key(&json!(42)), "42");
        assert_eq!(hex_lower(&[0, 10, 255]), "000aff");
        assert_eq!(hex_upper(&[0, 10, 255]), "00 0A FF");
        assert_eq!(bytes_value(&[0, 10, 255]), json!({"bytes_hex": "000aff"}));
    }

    #[test]
    fn binary_readers_read_endianness_and_report_bounds_errors() {
        let data = [0x01, 0x02, 0x03, 0x04, 0xff, 0xff, 0xff, 0xfe];

        assert_eq!(read_u16(&data, 0, false).unwrap(), 0x0102);
        assert_eq!(read_u16(&data, 0, true).unwrap(), 0x0201);
        assert_eq!(read_u32(&data, 0, false).unwrap(), 0x01020304);
        assert_eq!(read_u32(&data, 0, true).unwrap(), 0x04030201);
        assert_eq!(read_i32_be(&data, 4).unwrap(), -2);

        assert!(read_u16(&data, 7, false).is_err());
        assert!(read_u32(&data, 5, false).is_err());
        assert!(read_i32_be(&data, 5).is_err());
    }

    #[test]
    fn dotnet_ticks_convert_to_unix_millis_only_when_in_dotnet_range() {
        assert_eq!(dotnet_ticks_to_unix_millis(1_717_171_717), 1_717_171_717);
        assert_eq!(
            dotnet_ticks_to_unix_millis(DOTNET_EPOCH_TICKS + 1_234_0000),
            1234
        );
    }
}
