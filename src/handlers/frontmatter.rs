use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct FileList {
    pub files: Vec<String>,
}

/// Extract YAML frontmatter from markdown content
/// Returns (frontmatter_map, remaining_content)
pub fn extract_frontmatter(
    content: &str,
) -> Result<(HashMap<String, String>, String), ProcessError> {
    let content = content.trim_start();

    if !content.starts_with("---") {
        return Err(ProcessError {
            error_type: ErrorType::Syntax,
            item: WithItem::Data,
            path: PathBuf::new(),
            message: Some("Frontmatter must start with '---'".to_string()),
        });
    }

    let after_first_delimiter = &content[3..];

    if let Some(end_pos) = after_first_delimiter.find("\n---") {
        let frontmatter_str = &after_first_delimiter[..end_pos];
        let remaining = &after_first_delimiter[end_pos + 4..].trim_start();

        // Parse YAML frontmatter
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(frontmatter_str)
            .map_err(|e| ProcessError {
                error_type: ErrorType::Syntax,
                item: WithItem::Data,
                path: PathBuf::new(),
                message: Some(format!("Failed to parse YAML frontmatter: {}", e)),
            })?;

        // Convert YAML to HashMap<String, String>
        let mut map = HashMap::new();
        if let serde_yaml::Value::Mapping(mapping) = yaml_value {
            for (key, value) in mapping {
                if let serde_yaml::Value::String(k) = &key {
                    let v = match &value {
                        serde_yaml::Value::String(s) => s.clone(),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        _ => String::new(),
                    };
                    map.insert(k.clone(), v);
                }
            }
        }

        // Validate that title exists
        if !map.contains_key("title") {
            return Err(ProcessError {
                error_type: ErrorType::Syntax,
                item: WithItem::Data,
                path: PathBuf::new(),
                message: Some("Frontmatter must contain a 'title' field".to_string()),
            });
        }

        Ok((map, remaining.to_string()))
    } else {
        Err(ProcessError {
            error_type: ErrorType::Syntax,
            item: WithItem::Data,
            path: PathBuf::new(),
            message: Some("Frontmatter must end with '---'".to_string()),
        })
    }
}

/// Load data from markdown files with frontmatter based on a TOML file list
/// Returns a JSON array value compatible with the existing template system
pub fn load_frontmatter_data(
    src: &PathBuf,
    name: &str,
) -> Result<(Value, Vec<ProcessError>), Vec<ProcessError>> {
    let mut errors = Vec::new();

    let toml_path = src
        .join("data")
        .join(name.replace(":", "/"))
        .with_extension("data.toml");

    // Read the TOML file
    let toml_content = fs::read_to_string(&toml_path)
        .map_proc_err(
            WithItem::Data,
            ErrorType::Io,
            &toml_path,
            Some("Failed to read data.toml file".to_string()),
        )
        .map_err(|e| vec![e])?;

    // Parse the TOML file
    let file_list: FileList = toml::from_str(&toml_content).map_err(|e| {
        vec![ProcessError {
            error_type: ErrorType::Syntax,
            item: WithItem::Data,
            path: toml_path.clone(),
            message: Some(format!("Failed to parse TOML: {}", e)),
        }]
    })?;

    let data_dir = src.join("data").join(name.replace(":", "/"));
    let mut items = Vec::new();

    for file in &file_list.files {
        let md_path = data_dir.join(file);

        let content = match fs::read_to_string(&md_path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(ProcessError {
                    error_type: ErrorType::Io,
                    item: WithItem::Data,
                    path: md_path.clone(),
                    message: Some(format!("Failed to read markdown file: {}", e)),
                });
                continue;
            }
        };

        let (mut frontmatter, _) = match extract_frontmatter(&content) {
            Ok((fm, remaining)) => (fm, remaining),
            Err(mut e) => {
                e.path = md_path.clone();
                errors.push(e);
                continue;
            }
        };

        // Generate entry-path and result-path from filename
        let file_stem = md_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let relative_entry_path = format!("{}/{}", name.replace(":", "/"), file);
        let result_path = format!("content/{}.html", file_stem);

        // Add the special fields
        frontmatter.insert("--entry-path".to_string(), relative_entry_path);
        frontmatter.insert("--result-path".to_string(), result_path.clone());
        frontmatter.insert("link".to_string(), format!("./{}", result_path));

        // Convert HashMap to serde_json::Value
        let obj: serde_json::Map<String, Value> = frontmatter
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();

        items.push(Value::Object(obj));
    }

    Ok((Value::Array(items), errors))
}
