// src/packet/parameters.rs

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RawParameters {
    inner: BTreeMap<u8, Value>,
}

impl RawParameters {
    pub fn new(inner: BTreeMap<u8, Value>) -> Self {
        Self { inner }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn get(&self, key: u8) -> Option<&Value> {
        self.inner.get(&key)
    }

    pub fn insert(&mut self, key: u8, value: Value) -> Option<Value> {
        self.inner.insert(key, value)
    }

    pub fn as_map(&self) -> &BTreeMap<u8, Value> {
        &self.inner
    }

    pub fn into_inner(self) -> BTreeMap<u8, Value> {
        self.inner
    }

    pub fn to_serializable(&self) -> SerializableParameters {
        SerializableParameters::from_raw(self)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SerializableParameters {
    inner: BTreeMap<String, Value>,
}

impl SerializableParameters {
    pub fn from_raw(parameters: &RawParameters) -> Self {
        let inner = parameters
            .as_map()
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect();

        Self { inner }
    }

    pub fn as_map(&self) -> &BTreeMap<String, Value> {
        &self.inner
    }

    pub fn into_inner(self) -> BTreeMap<String, Value> {
        self.inner
    }
}