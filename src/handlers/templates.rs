use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use crate::handlers::entries::process_entry;
use crate::handlers::frontmatter::load_frontmatter_data;
use crate::handlers::pages::page;
use crate::utils::kv_replace;
use crate::utils::ProcessResult;
use fancy_regex::Regex;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::{collections::HashSet, fs, path::PathBuf, str};

static TEMPLATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?<!<!--)<::Template\{([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)\}\s*\/>(?!.*?-->)"#)
        .expect("Regex failed to parse. This shouldn't happen.")
});

pub fn get_template(src: &PathBuf, name: &str, mut hist: HashSet<PathBuf>) -> ProcessResult {
    let mut errors: Vec<ProcessError> = Vec::new();
    let template_path = src
        .join("templates")
        .join(name.replace(":", "/"))
        .with_extension("template.html");

    let data_path = src
        .join("data")
        .join(name.replace(":", "/"))
        .with_extension("data.json");

    if !hist.insert(template_path.clone()) {
        return ProcessResult {
            output: String::new(),
            errors: vec![ProcessError {
                error_type: ErrorType::Circular,
                item: WithItem::Template,
                path: template_path,
                message: Some(format!("{:?}", hist)),
            }],
        };
    }

    let template = match fs::read_to_string(&template_path).map_proc_err(
        WithItem::Template,
        ErrorType::Io,
        &template_path,
        Some("Failed to read template file".to_string()),
    ) {
        Ok(content) => content,
        Err(e) => {
            return ProcessResult {
                output: String::new(),
                errors: vec![e],
            };
        }
    };

    if template.is_empty() {
        errors.push(ProcessError {
            error_type: ErrorType::Other,
            item: WithItem::Template,
            path: template_path.clone(),
            message: Some("Template file is empty".to_string()),
        });
        return ProcessResult {
            output: String::new(),
            errors,
        };
    }

    // Try to load from .toml + frontmatter first, fall back to .json
    let toml_path = src
        .join("data")
        .join(name.replace(":", "/"))
        .with_extension("data.toml");

    let v: Value = if toml_path.exists() {
        // Use frontmatter-based loading
        match load_frontmatter_data(src, name) {
            Ok((value, fm_errors)) => {
                errors.extend(fm_errors);
                value
            }
            Err(fm_errors) => {
                errors.extend(fm_errors);
                return ProcessResult {
                    output: String::new(),
                    errors,
                };
            }
        }
    } else {
        // Fall back to JSON loading
        let data = match fs::read_to_string(&data_path).map_proc_err(
            WithItem::Data,
            ErrorType::Io,
            &data_path,
            Some("Failed to read data file".to_string()),
        ) {
            Ok(content) => content,
            Err(e) => {
                return ProcessResult {
                    output: String::new(),
                    errors: vec![e],
                };
            }
        };

        if data.is_empty() {
            errors.push(ProcessError {
                error_type: ErrorType::Other,
                item: WithItem::Data,
                path: data_path.clone(),
                message: Some("Data file is empty".to_string()),
            });
            return ProcessResult {
                output: String::new(),
                errors,
            };
        }

        match serde_json::from_str(&data) {
            Ok(value) => value,
            Err(e) => {
                errors.push(ProcessError {
                    error_type: ErrorType::Syntax,
                    item: WithItem::Data,
                    path: data_path,
                    message: Some(format!("JSON decode error: {}", e)),
                });
                return ProcessResult {
                    output: String::new(),
                    errors,
                };
            }
        }
    };

    let items = match v.as_array() {
        Some(array) => array,
        None => {
            errors.push(ProcessError {
                error_type: ErrorType::Syntax,
                item: WithItem::Data,
                path: data_path,
                message: Some("JSON wasn't an array".to_string()),
            });
            return ProcessResult {
                output: String::new(),
                errors,
            };
        }
    };

    let mut contents = String::with_capacity(template.len() * items.len());

    for object in items {
        let obj = match object.as_object() {
            Some(obj) => obj,
            None => {
                errors.push(ProcessError {
                    error_type: ErrorType::Syntax,
                    item: WithItem::Data,
                    path: data_path.clone(),
                    message: Some("Invalid object in JSON".to_string()),
                });
                continue;
            }
        };

        let mut entry_path = String::new();
        let mut result_path = String::new();
        let mut is_entry = false;
        let mut kv: Vec<(&str, &str)> = Vec::with_capacity(obj.len());

        for (key, value) in obj {
            let val = match value.as_str() {
                Some(s) => s,
                None => {
                    errors.push(ProcessError {
                        error_type: ErrorType::Syntax,
                        item: WithItem::Data,
                        path: data_path.clone(),
                        message: Some(format!(
                            "Value for key '{}' couldn't be decoded to string",
                            key
                        )),
                    });
                    continue;
                }
            };

            match key.as_str() {
                "--entry-path" => {
                    entry_path = val.to_string();
                    is_entry = true;
                }
                "--result-path" => {
                    result_path = val.to_string();
                    is_entry = true;
                }
                _ => {}
            }
            kv.push((key, val));
        }

        let processed_template = kv_replace(kv.clone(), template.clone());
        contents.push_str(&processed_template);

        if is_entry {
            let entry_errs = process_entry(src, name, entry_path, result_path, kv);
            errors.extend(entry_errs);
        }
    }

    let page_res = page(src, contents, hist);
    errors.extend(page_res.errors);
    ProcessResult {
        output: page_res.output,
        errors,
    }
}

pub fn process_template(src: &PathBuf, input: String, hist: HashSet<PathBuf>) -> ProcessResult {
    let mut errors = Vec::new();
    let mut output = input;

    // Early return if no templates
    if !TEMPLATE_REGEX.is_match(&output).unwrap_or(false) {
        return ProcessResult { output, errors };
    }

    let mut replacements = Vec::new();

    for f in TEMPLATE_REGEX.find_iter(&output) {
        match f {
            Ok(found) => {
                let found_str = found.as_str();
                let template_name = match found_str
                    .trim()
                    .strip_prefix("<::Template{")
                    .and_then(|s| s.strip_suffix("/>"))
                    .map(|s| s.trim().trim_end_matches('}'))
                {
                    Some(name) if !name.is_empty() => name,
                    _ => {
                        errors.push(ProcessError {
                            error_type: ErrorType::Syntax,
                            item: WithItem::Template,
                            path: PathBuf::new(),
                            message: Some(format!(
                                "Malformed template tag: '{}'. Expected <::Template{{Name}} />",
                                found_str
                            )),
                        });
                        continue;
                    }
                };

                let result = get_template(src, template_name, hist.clone());
                errors.extend(result.errors);
                replacements.push((found_str.to_owned(), result.output));
            }
            Err(e) => {
                errors.push(ProcessError {
                    error_type: ErrorType::Other,
                    item: WithItem::Template,
                    path: PathBuf::new(),
                    message: Some(format!("Regex error while scanning for templates: {}", e)),
                });
            }
        }
    }

    for (old, new) in replacements.into_iter().rev() {
        output = output.replacen(&old, &new, 1);
    }

    ProcessResult { output, errors }
}
