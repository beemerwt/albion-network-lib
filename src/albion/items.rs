use crate::error::Result;
use std::collections::HashMap;

const ITEM_NAME_MAPPINGS_URL: &str =
    "https://cdn.albionfreemarket.com/AlbionFormattedItemsParser/us_name_mappings.json";

#[derive(Clone, Debug, Default)]
pub struct ItemNameResolver {
    names_by_id: HashMap<String, String>,
}

impl ItemNameResolver {
    pub fn new(names_by_id: HashMap<String, String>) -> Self {
        Self { names_by_id }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let names_by_id = serde_json::from_str(json)?;
        Ok(Self::new(names_by_id))
    }

    pub fn download_default() -> Result<Self> {
        Self::download_from(ITEM_NAME_MAPPINGS_URL)
    }

    pub fn download_from(url: &str) -> Result<Self> {
        let response = ureq::get(url)
            .call()
            .map_err(|err| format!("failed to download item name mappings: {err}"))?;

        let text = response
            .into_string()
            .map_err(|err| format!("failed to read item name mappings response: {err}"))?;

        Self::from_json(&text)
    }

    pub fn resolve(&self, item_id: &str) -> Option<&str> {
        self.names_by_id.get(item_id).map(String::as_str)
    }

    pub fn resolve_owned(&self, item_id: &str) -> Option<String> {
        self.resolve(item_id).map(str::to_owned)
    }

    pub fn len(&self) -> usize {
        self.names_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names_by_id.is_empty()
    }
}
