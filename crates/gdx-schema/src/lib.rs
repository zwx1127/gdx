use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SceneSpec {
    pub root: SceneNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SceneNode {
    #[serde(rename = "type")]
    pub type_name: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SceneNode>,
}

impl SceneSpec {
    pub fn validate_minimal(&self) -> Result<(), String> {
        validate_node(&self.root)
    }
}

fn validate_node(node: &SceneNode) -> Result<(), String> {
    if node.type_name.trim().is_empty() {
        return Err("Scene node type must not be empty".to_string());
    }
    if node.name.trim().is_empty() {
        return Err("Scene node name must not be empty".to_string());
    }
    for child in &node.children {
        validate_node(child)?;
    }
    Ok(())
}
