use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WasmComponentMetadata {
    pub name: String,
    pub wasm: PathBuf,
    #[serde(default)]
    pub env: Vec<EnvironmentVariable>,
    #[serde(default)]
    pub args: Vec<String>,
}

impl WasmComponentMetadata {
    /// Load module metadata from a YAML file
    pub fn load_from_yaml(path: &PathBuf) -> Result<Vec<WasmComponentMetadata>> {
        let contents = fs::read_to_string(path)?;

        contents
            .split("\n---")
            .filter_map(|yaml_doc| {
                match serde_yml::from_str::<WasmComponentMetadata>(yaml_doc) {
                    Err(err) if err.to_string().contains("EOF while parsing a value") => None,
                    result => Some(
                        result.map_err(|e| anyhow::anyhow!("Failed to parse module: {}", e)),
                    ),
                }
            })
            .collect()
    }
}

