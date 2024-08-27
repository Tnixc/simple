use crate::error::{ErrorType, MapPageError, ProcessError, WithItem};
use crate::handlers::pages::page;
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
) -> Result<String, ProcessError> {
    let template_path = src
        .join("templates")
        .join(name.replace(":", "/"))
        .with_extension("template.html");

    let data_path = src
        .join("data")
        .join(name.replace(":", "/"))
        .with_extension("data.json");

    let template_content_utf = fs::read(&template_path).map_page_err(
        WithItem::Template,
        ErrorType::NotFound,
        &template_path,
    )?;
    let template = String::from_utf8(template_content_utf).map_page_err(
        WithItem::Template,
        ErrorType::Utf8,
        &template_path,
    )?;

    let data_content_utf8 =
        fs::read(&data_path).map_page_err(WithItem::Data, ErrorType::NotFound, &data_path)?;
    let data_str = str::from_utf8(&data_content_utf8).unwrap();
    let v: Value = serde_json::from_str(data_str).expect("JSON decode error");
    let items = v.as_array().expect("JSON wasn't an array");

    let mut contents = String::new();
    for object in items {
        let mut this = template.clone();
        for (key, value) in object.as_object().expect("Invalid object in JSON") {
            let key = format!("${{{key}}}");
            this = this.replace(
                key.as_str(),
                value
                    .as_str()
                    .expect("JSON object value couldn't be decoded to string"),
            );
        }
        contents.push_str(&this);
    }
    if !hist.insert(template_path.clone()) {
        return Err(ProcessError {
            error_type: ErrorType::Circular,
            item: WithItem::Template,
            path_or_message: template_path,
        });
    }
    return page(src, contents.into_bytes(), false, hist);
}

pub fn process_template(
    src: &PathBuf,
    input: String,
    hist: HashSet<PathBuf>,
) -> Result<String, ProcessError> {
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

            let replacement = get_template(src, template_name, hist.clone())?;
            replacements.push((found.as_str().to_string(), replacement));
        }
    }

    for (old, new) in replacements.into_iter().rev() {
        output = output.replacen(&old, &new, 1);
    }

    Ok(output)
}
