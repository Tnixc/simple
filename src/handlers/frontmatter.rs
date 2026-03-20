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

/// Result of frontmatter extraction: the key-value map, remaining content, and any warnings.
pub struct FrontmatterResult {
    pub map: HashMap<String, String>,
    pub remaining: String,
    pub warnings: Vec<ProcessError>,
}

/// Extract YAML frontmatter from markdown content.
/// Returns the frontmatter map, remaining content, and any non-fatal warnings
/// (e.g. unsupported value types that were skipped).
pub fn extract_frontmatter(
    content: &str,
    path: &PathBuf,
) -> Result<FrontmatterResult, ProcessError> {
    let content = content.trim_start();

    if !content.starts_with("---") {
        return Err(ProcessError {
            error_type: ErrorType::Syntax,
            item: WithItem::Data,
            path: path.clone(),
            message: Some("Frontmatter must start with '---'".to_string()),
        });
    }

    let after_first_delimiter = &content[3..];

    if let Some(end_pos) = after_first_delimiter.find("\n---") {
        let frontmatter_str = &after_first_delimiter[..end_pos];
        let remaining = &after_first_delimiter[end_pos + 4..].trim_start();

        // Parse YAML frontmatter
        let yaml_value: serde_yaml::Value =
            serde_yaml::from_str(frontmatter_str).map_err(|e| ProcessError {
                error_type: ErrorType::Syntax,
                item: WithItem::Data,
                path: path.clone(),
                message: Some(format!("Failed to parse YAML frontmatter: {}", e)),
            })?;

        let mut map = HashMap::new();
        let mut warnings = Vec::new();

        if let serde_yaml::Value::Mapping(mapping) = yaml_value {
            for (key, value) in mapping {
                if let serde_yaml::Value::String(k) = &key {
                    let v = match &value {
                        serde_yaml::Value::String(s) => s.clone(),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        serde_yaml::Value::Null => {
                            warnings.push(ProcessError {
                                error_type: ErrorType::Syntax,
                                item: WithItem::Data,
                                path: path.clone(),
                                message: Some(format!(
                                    "Frontmatter key '{}' has a null value and was skipped",
                                    k
                                )),
                            });
                            continue;
                        }
                        other => {
                            let type_name = match other {
                                serde_yaml::Value::Sequence(_) => "array",
                                serde_yaml::Value::Mapping(_) => "nested mapping",
                                serde_yaml::Value::Tagged(_) => "tagged value",
                                _ => "unsupported type",
                            };
                            warnings.push(ProcessError {
                                error_type: ErrorType::Syntax,
                                item: WithItem::Data,
                                path: path.clone(),
                                message: Some(format!(
                                    "Frontmatter key '{}' has an unsupported type ({}) and was skipped. \
                                     Only strings, numbers, and booleans are supported.",
                                    k, type_name
                                )),
                            });
                            continue;
                        }
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
                path: path.clone(),
                message: Some("Frontmatter must contain a 'title' field".to_string()),
            });
        }

        Ok(FrontmatterResult {
            map,
            remaining: remaining.to_string(),
            warnings,
        })
    } else {
        Err(ProcessError {
            error_type: ErrorType::Syntax,
            item: WithItem::Data,
            path: path.clone(),
            message: Some("Frontmatter must end with '---'".to_string()),
        })
    }
}

/// Load data from markdown files with frontmatter based on a TOML file list.
/// Returns a JSON array value compatible with the existing template system.
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

        let fm_result = match extract_frontmatter(&content, &md_path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };

        // Collect frontmatter warnings
        errors.extend(fm_result.warnings);

        let mut frontmatter = fm_result.map;

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
