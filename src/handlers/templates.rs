use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use crate::handlers::entries::process_entry;
use crate::handlers::pages::page;
use crate::utils::kv_replace;
use crate::utils::ProcessResult;
use fancy_regex::Regex;
use lazy_static::lazy_static;
use serde_json::Value;
use std::{collections::HashSet, fs, path::PathBuf, str};

const TEMPLATE_PATTERN: &str =
    r#"(?<!<!--)<-Template\{([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)\}\s*\/>(?!.*?-->)"#;

lazy_static! {
    static ref TEMPLATE_REGEX: Regex =
        Regex::new(TEMPLATE_PATTERN).expect("Regex failed to parse. This shouldn't happen.");
}

pub fn get_template(
    src: &PathBuf,
    name: &str,
    mut hist: HashSet<PathBuf>,
    dev: bool,
) -> ProcessResult {
    let mut errors: Vec<ProcessError> = Vec::new();
    let template_path = src
        .join("templates")
        .join(name.replace(":", "/"))
        .with_extension("template.html");

    let data_path = src
        .join("data")
        .join(name.replace(":", "/"))
        .with_extension("data.json");

    let template = fs::read_to_string(&template_path)
        .map_proc_err(
            WithItem::Template,
            ErrorType::Io,
            &template_path,
            Some("Failed to read template file".to_string()),
        )
        .inspect_err(|e| errors.push((*e).clone()))
        .unwrap_or(String::new());

    let data = fs::read_to_string(&data_path)
        .map_proc_err(
            WithItem::Data,
            ErrorType::Io,
            &data_path,
            Some("Failed to read data file".to_string()),
        )
        .inspect_err(|e| errors.push((*e).clone()))
        .unwrap_or(String::new());

    let v: Value = serde_json::from_str(&data).expect("JSON decode error");
    let items = v.as_array().expect("JSON wasn't an array");

    let mut contents = String::new();

    for object in items {
        let mut entry_path = String::new();
        let mut result_path = String::new();
        let mut is_entry = false;
        let mut kv: Vec<(&str, &str)> = Vec::new();
        for (key, value) in object.as_object().expect("Invalid object in JSON") {
            match key.as_str() {
                "--entry-path" => {
                    entry_path = value
                        .as_str()
                        .expect("JSON object value couldn't be decoded to string")
                        .to_string();
                    is_entry = true;
                }
                "--result-path" => {
                    result_path = value
                        .as_str()
                        .expect("JSON object value couldn't be decoded to string")
                        .to_string();
                    is_entry = true;
                }
                _ => {}
            }
            let val = value
                .as_str()
                .expect("JSON object value couldn't be decoded to string");
            kv.push((key, val));
        }

        let this = kv_replace(kv.clone(), template.clone());
        contents.push_str(&this);

        if is_entry {
            let entry_errs = process_entry(
                src,
                name,
                entry_path.to_string(),
                result_path.to_string(),
                kv,
                hist.clone(),
                dev,
            );
            errors.extend(entry_errs);
        }
    }

    if !hist.insert(template_path.clone()) {
        return ProcessResult {
            output: String::new(),
            errors: vec![ProcessError {
                error_type: ErrorType::Circular,
                item: WithItem::Component,
                path: PathBuf::from(template_path),
                message: Some(format!("{:?}", hist)),
            }],
        };
    }

    let page_res = page(src, contents, dev, hist);
    errors.extend(page_res.errors);
    return ProcessResult {
        output: page_res.output,
        errors,
    };
}

pub fn process_template(
    src: &PathBuf,
    input: String,
    hist: HashSet<PathBuf>,
    dev: bool,
) -> ProcessResult {
    let mut errors = Vec::new();
    let mut replacements = Vec::new();

    let mut output = input;
    for f in TEMPLATE_REGEX.find_iter(output.as_str()) {
        if let Ok(found) = f {
            let template_name = found
                .as_str()
                .trim()
                .trim_start_matches("<-Template{")
                .trim_end_matches("/>")
                .trim()
                .trim_end_matches("}");

            let result = get_template(src, template_name, hist.clone(), dev);
            let replacement = result.output;
            errors.extend(result.errors);
            replacements.push((found.as_str().to_string(), replacement));
        }
    }

    for (old, new) in replacements.into_iter().rev() {
        output = output.replacen(&old, &new, 1);
    }

    return ProcessResult { output, errors };
}
