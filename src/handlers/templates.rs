use crate::dev::SCRIPT;
use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use crate::handlers::pages::page;
use crate::utils::kv_replace;
use crate::utils::ProcessResult;
use fancy_regex::Regex;
use lazy_static::lazy_static;
use minify_html::minify;
use serde_json::Value;
use std::{collections::HashSet, fs, path::PathBuf, str};

const TEMPLATE_PATTERN: &str =
    r#"(?<!<!--)<-Template\{([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)\}\s*\/>(?!.*?-->)"#;

lazy_static! {
    static ref TEMPLATE_REGEX: Regex =
        Regex::new(TEMPLATE_PATTERN).expect("Regex failed to parse. This shouldn't happen.");
}

pub fn process_entry(
    src: &PathBuf,
    name: &str,
    entry_path: String,
    result_path: String,
    kv: Vec<(String, String)>,
    hist: HashSet<PathBuf>,
    dev: bool,
) -> Vec<ProcessError> {
    let mut errors: Vec<ProcessError> = Vec::new();

    if entry_path.is_empty() || result_path.is_empty() {
        return vec![ProcessError {
            error_type: ErrorType::Other,
            item: WithItem::Template,
            path: PathBuf::from(result_path),
            message: Some(
                format!("Error occurred in {name}. The --entry-path and --result-path keys must both be present if either is present.")
            ),
        }];
    }

    let entry_path = src.join("data").join(entry_path.trim_start_matches("/"));
    let frame_path = src
        .join("templates")
        .join(name.replace(":", "/"))
        .with_extension("frame.html");
    let result_path = src
        .parent()
        .unwrap()
        .join({
            if dev {
                "dev"
            } else {
                "dist"
            }
        })
        .join(result_path.trim_start_matches("/"));

    let frame_content = match fs::read_to_string(&frame_path) {
        Ok(content) => content,
        Err(e) => {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::Data,
                path: frame_path.clone(),
                message: Some(format!("Failed to read frame file: {}", e)),
            });
            return errors;
        }
    };

    let content = match fs::read_to_string(&entry_path) {
        Ok(content) => content,
        Err(e) => {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::Data,
                path: entry_path.clone(),
                message: Some(format!("Failed to read data file: {}", e)),
            });
            return errors;
        }
    };

    let processed_content = if entry_path.extension().and_then(|s| s.to_str()) == Some("md") {
        frame_content.replace(
            "${--content}",
            &("<markdown>".to_owned() + &content + "</markdown>"),
        )
    } else {
        frame_content.replace("${--content}", &content)
    };

    let final_content = kv_replace(
        kv.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect(),
        processed_content,
    );

    let page_result = page(src, final_content, false, hist);

    if let Some(parent) = result_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: parent.to_path_buf(),
                message: Some(format!("Failed to create directory structure: {}", e)),
            });
            return errors;
        }
    }

    let mut output: Vec<u8>;
    if dev {
        if page_result.output.contains("</head>") {
            let modified_output = page_result.output.replace(
                "</head>",
                &format!("<script src=\"{}\"></script></head>", SCRIPT),
            );
            output = modified_output.into_bytes();
        } else {
            let modified_output = format!("<head>{}</head>{}", SCRIPT, page_result.output);
            output = modified_output.into_bytes();
        }
    } else {
        output = page_result.output.into_bytes();
        output = minify(&output, &minify_html::Cfg::spec_compliant());
    }

    match fs::write(&result_path, &output) {
        Ok(_) => (),
        Err(e) => {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: result_path.clone(),
                message: Some(format!("Failed to write result file: {}", e)),
            });
        }
    }

    errors.extend(page_result.errors);
    errors
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
        let mut this = template.clone();
        let mut entry_path = String::new();
        let mut result_path = String::new();
        let mut is_entry = false;
        let mut kv: Vec<(String, String)> = Vec::new();
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
            let key = format!("${{{key}}}");
            let val = value
                .as_str()
                .expect("JSON object value couldn't be decoded to string")
                .to_string();
            kv.push((key, val));
        }

        for (key, value) in kv.iter() {
            this = this.replace(key, value);
        }

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
    return page(src, contents, false, hist);
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
