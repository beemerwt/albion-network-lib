use std::collections::HashMap;

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

    pub fn resolve(&self, item_id: &str) -> Option<&str> {
        self.names_by_id.get(item_id).map(String::as_str)
    }

    pub fn resolve_owned(&self, item_id: &str) -> Option<String> {
        self.resolve(item_id).map(str::to_owned)
    }
}