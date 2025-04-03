use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use crate::handlers::pages::page;
use crate::utils::{get_inside, get_targets_kv, kv_replace, ProcessResult};
use fancy_regex::Regex;
use lazy_static::lazy_static;
use std::{collections::HashSet, fs, path::PathBuf};

const COMPONENT_PATTERN_SELF: &str =
    r#"(?<!<!--)<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['\"]).*?\4)*\s*\/>(?!.*?-->)"#;

const COMPONENT_PATTERN_WRAPPING: &str =
    r#"(?<!<!--)<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['\"]).*?\4)*\s*>(?!.*?-->)"#;

const SLOT_PATTERN: &str = r#"(?<!<!--)<slot([\S\s])*>*?<\/slot>(?!.*?-->)"#;

lazy_static! {
    static ref REGEX_SELF_CLOSING: Regex =
        Regex::new(COMPONENT_PATTERN_SELF).expect("Regex failed to parse. This shouldn't happen.");
    static ref REGEX_WRAPPING: Regex = Regex::new(COMPONENT_PATTERN_WRAPPING)
        .expect("Regex failed to parse. This shouldn't happen.");
    static ref REGEX_SLOT: Regex =
        Regex::new(SLOT_PATTERN).expect("Regex failed to parse. This shouldn't happen.");
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
) -> ProcessResult {
    let mut errors: Vec<ProcessError> = Vec::new();
    let path = src
        .join("components")
        .join(component.replace(":", "/"))
        .with_extension("component.html");

    let mut st = fs::read_to_string(&path)
        .map_proc_err(WithItem::Component, ErrorType::Io, &path, None)
        .inspect_err(|e| errors.push((*e).clone()))
        .unwrap_or(String::new());

    st = kv_replace(targets, st);
    if !hist.insert(path.clone()) {
        return ProcessResult {
            output: String::new(),
            errors: vec![ProcessError {
                error_type: ErrorType::Circular,
                item: WithItem::Component,
                path: PathBuf::from(path),
                message: Some(format!("{:?}", hist)),
            }],
        };
    }
    let result = page(src, st, hist);
    errors.extend(result.errors);
    return ProcessResult {
        output: result.output,
        errors,
    };
}

pub fn get_component_slot(
    src: &PathBuf,
    component: &str,
    targets: Vec<(&str, &str)>,
    slot_content: Option<String>,
    mut hist: HashSet<PathBuf>,
) -> ProcessResult {
    let mut errors: Vec<ProcessError> = Vec::new();
    let path = src
        .join("components")
        .join(component.replace(":", "/"))
        .with_extension("component.html");
    let mut st = fs::read_to_string(&path)
        .map_proc_err(WithItem::Component, ErrorType::Io, &path, None)
        .inspect_err(|e| errors.push((*e).clone()))
        .unwrap_or(String::new());

    if !st.contains("<slot>") || !st.contains("</slot>") {
        return ProcessResult {
            output: String::new(),
            errors: vec![ProcessError {
                error_type: ErrorType::Syntax,
                item: WithItem::Component,
                path,
                message: Some(String::from(
                    "The component does not contain a proper <slot></slot> tag.",
                )),
            }],
        };
    }

    st = kv_replace(targets.clone(), st);
    if let Some(content) = slot_content {
        // here it replaces "<slot>fallback</slot>" with "<slot></slot>, after the content is exists"
        for find in REGEX_SLOT.find_iter(&st.clone()) {
            st = st.replace(&find.unwrap().as_str(), &content);
        }
    }
    if !hist.insert(path.clone()) {
        return ProcessResult {
            output: String::new(),
            errors: vec![ProcessError {
                error_type: ErrorType::Circular,
                item: WithItem::Component,
                path: PathBuf::from(path),
                message: Some(format!("{:?}", hist)),
            }],
        };
    }

    let result = page(src, st, hist);
    errors.extend(result.errors);
    return ProcessResult {
        output: result.output,
        errors,
    };
}

pub fn process_component(
    src: &PathBuf,
    input: String,
    component_type: ComponentTypes,
    hist: HashSet<PathBuf>,
) -> ProcessResult {
    let regex = match component_type {
        ComponentTypes::SelfClosing => &*REGEX_SELF_CLOSING,
        ComponentTypes::Wrapping => &*REGEX_WRAPPING,
    };

    let mut errors: Vec<ProcessError> = Vec::new();

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
            let targets = get_targets_kv(name, found.as_str())
                .inspect_err(|e| errors.push((*e).clone()))
                .unwrap_or(Vec::new());
            match component_type {
                ComponentTypes::SelfClosing => {
                    let target = found.as_str();
                    let result = get_component_self(src, name, targets, hist.clone());
                    let replacement = result.output;
                    errors.extend(result.errors);

                    output = output.replacen(target, &replacement, 1);
                }
                ComponentTypes::Wrapping => {
                    let end = format!("</{}>", &name);
                    let slot_content = get_inside(output.clone(), found.as_str(), &end);
                    let result =
                        get_component_slot(src, name, targets, slot_content.clone(), hist.clone());
                    let replacement = result.output;
                    errors.extend(result.errors);

                    output =
                        output.replacen(slot_content.unwrap_or("".to_string()).as_str(), "", 1);
                    output = output.replacen(&end, "", 1);
                    output = output.replacen(found.as_str(), &replacement, 1);
                }
            }
        }
    }
    return ProcessResult { output, errors };
}
