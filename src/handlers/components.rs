use crate::error::{ErrorType, MapPageError, ProcessError, WithItem};
use crate::handlers::pages::page;
use crate::utils::{get_inside, get_targets_kv, kv_replace};
use color_print::cformat;
use fancy_regex::Regex;
use std::{collections::HashSet, fs, path::PathBuf};
use lazy_static::lazy_static;

const COMPONENT_PATTERN_SELF: &str =
    r#"(?<!<!--)<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['\"]).*?\4)*\s*\/>(?!.*?-->)"#;

const COMPONENT_PATTERN_WRAPPING: &str =
    r#"(?<!<!--)<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['\"]).*?\4)*\s*>(?!.*?-->)"#;

const SLOT_PATTERN: &str = r#"(?<!<!--)<slot([\S\s])*>*?<\/slot>(?!.*?-->)"#;

lazy_static! {
    static ref REGEX_SELF_CLOSING: Regex = Regex::new(COMPONENT_PATTERN_SELF)
        .expect("Regex failed to parse. This shouldn't happen.");
    static ref REGEX_WRAPPING: Regex = Regex::new(COMPONENT_PATTERN_WRAPPING)
        .expect("Regex failed to parse. This shouldn't happen.");
    static ref REGEX_SLOT: Regex = Regex::new(SLOT_PATTERN)
        .expect("Regex failed to parse. This shouldn't happen.");
}

pub enum ComponentTypes {
    SelfClosing,
    Wrapping,
}


pub fn get_component_self(
    src: &PathBuf,
    component: &str,
    targets: Vec<(&str, &str)>,
    mut hist: HashSet<PathBuf>,
) -> Result<String, ProcessError> {
    let path = src
        .join("components")
        .join(component.replace(":", "/"))
        .with_extension("component.html");

    let v = fs::read(&path).map_page_err(WithItem::Component, ErrorType::NotFound, &path)?;
    let mut st = String::from_utf8(v).map_page_err(WithItem::Component, ErrorType::Utf8, &path)?;
    st = kv_replace(targets, st);
    let contents = st.clone().into_bytes();
    if !hist.insert(path.clone()) {
        return Err(ProcessError {
            error_type: ErrorType::Circular,
            item: WithItem::Component,
            path_or_message: PathBuf::from(path),
        });
    }
    return page(src, contents, false, hist);
}

pub fn get_component_slot(
    src: &PathBuf,
    component: &str,
    targets: Vec<(&str, &str)>,
    slot_content: Option<String>,
    mut hist: HashSet<PathBuf>,
) -> Result<String, ProcessError> {
    let path = src
        .join("components")
        .join(component.replace(":", "/"))
        .with_extension("component.html");
    let v = fs::read(&path).map_page_err(WithItem::Component, ErrorType::NotFound, &path)?;
    let mut st = String::from_utf8(v).expect("Contents of component is not UTF8");

    if !st.contains("<slot>") || !st.contains("</slot>") {
        let msg = cformat!(
            "The component <r>{}</> does not contain a proper slot tag",
            path.to_str().unwrap()
        );
        return Err(ProcessError {
            error_type: ErrorType::Syntax,
            item: WithItem::Component,
            path_or_message: PathBuf::from(msg),
        });
    }

    st = kv_replace(targets, st);
    if let Some(content) = slot_content {
        // here it replaces "<slot>fallback</slot>" with "<slot></slot>, after the content is exists"
        st = REGEX_SLOT.replace(&st, &content).to_string();
    }

    if !hist.insert(path.clone()) {
        return Err(ProcessError {
            error_type: ErrorType::Circular,
            item: WithItem::Component,
            path_or_message: PathBuf::from(path),
        });
    }

    return page(src, st.into_bytes(), false, hist);
}

pub fn process_component(
    src: &PathBuf,
    input: String,
    component_type: ComponentTypes,
    hist: HashSet<PathBuf>,
) -> Result<String, ProcessError> {
    let regex = match component_type {
        ComponentTypes::SelfClosing => &*REGEX_SELF_CLOSING,
        ComponentTypes::Wrapping => &*REGEX_WRAPPING,
    };

    let mut output = input;
    for f in regex.find_iter(output.clone().as_str()) {
        if let Ok(found) = f {
            let trim = found
                .as_str()
                .trim()
                .trim_start_matches("<")
                .trim_end_matches("/>")
                .trim_end_matches(">")
                .trim();
            let name = trim.split_whitespace().next().unwrap_or(trim);
            let targets = get_targets_kv(name, found.as_str())?;

            match component_type {
                ComponentTypes::SelfClosing => {
                    let target = found.as_str();
                    let replacement = get_component_self(src, name, targets, hist.clone())?;
                    output = output.replacen(target, &replacement, 1);
                }
                ComponentTypes::Wrapping => {
                    let end = format!("</{}>", &name);
                    let slot_content = get_inside(output.clone(), found.as_str(), &end);
                    let replacement =
                        get_component_slot(src, name, targets, slot_content.clone(), hist.clone())?;
                    output =
                        output.replacen(slot_content.unwrap_or("".to_string()).as_str(), "", 1);
                    output = output.replacen(&end, "", 1);
                    output = output.replacen(found.as_str(), &replacement, 1);
                }
            }
        }
    }
    Ok(output)
}
