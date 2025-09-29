use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use crate::handlers::pages::page;
use crate::utils::{get_inside, get_targets_kv, kv_replace, ProcessResult};
use fancy_regex::Regex;
use once_cell::sync::Lazy;
use std::{collections::HashSet, fs, path::PathBuf};

static REGEX_SELF_CLOSING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?<!<!--)<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['\"]).*?\4)*\s*\/>(?!.*?-->)"#)
        .expect("Regex failed to parse. This shouldn't happen.")
});

static REGEX_WRAPPING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?<!<!--)<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['\"]).*?\4)*\s*>(?!.*?-->)"#)
        .expect("Regex failed to parse. This shouldn't happen.")
});

static REGEX_SLOT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?<!<!--)<slot([\S\s])*>*?<\/slot>(?!.*?-->)"#)
        .expect("Regex failed to parse. This shouldn't happen.")
});

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

    if !hist.insert(path.clone()) {
        return ProcessResult {
            output: String::new(),
            errors: vec![ProcessError {
                error_type: ErrorType::Circular,
                item: WithItem::Component,
                path,
                message: Some(format!("{:?}", hist)),
            }],
        };
    }

    let mut st = fs::read_to_string(&path)
        .map_proc_err(WithItem::Component, ErrorType::Io, &path, None)
        .inspect_err(|e| errors.push((*e).clone()))
        .unwrap_or_else(|_| String::new());

    if st.is_empty() && !errors.is_empty() {
        return ProcessResult {
            output: String::new(),
            errors,
        };
    }

    st = kv_replace(targets, st);
    let result = page(src, st, hist);
    errors.extend(result.errors);
    ProcessResult {
        output: result.output,
        errors,
    }
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

    if !hist.insert(path.clone()) {
        return ProcessResult {
            output: String::new(),
            errors: vec![ProcessError {
                error_type: ErrorType::Circular,
                item: WithItem::Component,
                path,
                message: Some(format!("{:?}", hist)),
            }],
        };
    }

    let mut st = fs::read_to_string(&path)
        .map_proc_err(WithItem::Component, ErrorType::Io, &path, None)
        .inspect_err(|e| errors.push((*e).clone()))
        .unwrap_or_else(|_| String::new());

    if st.is_empty() && !errors.is_empty() {
        return ProcessResult {
            output: String::new(),
            errors,
        };
    }

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

    st = kv_replace(targets, st);
    if let Some(content) = slot_content {
        let mut result = String::with_capacity(st.len() + content.len());
        let mut last_end = 0;

        for find in REGEX_SLOT.find_iter(&st) {
            if let Ok(mat) = find {
                result.push_str(&st[last_end..mat.start()]);
                result.push_str(&content);
                last_end = mat.end();
            }
        }
        result.push_str(&st[last_end..]);
        st = result;
    }

    let result = page(src, st, hist);
    errors.extend(result.errors);
    ProcessResult {
        output: result.output,
        errors,
    }
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
    let mut replacements = Vec::new();

    for f in regex.find_iter(&output) {
        if let Ok(found) = f {
            let found_str = found.as_str();
            let trim = found_str
                .trim()
                .strip_prefix('<')
                .unwrap_or(found_str)
                .trim_end_matches("/>")
                .trim_end_matches('>')
                .trim();

            let name = trim.split_whitespace().next().unwrap_or(trim);
            let targets = get_targets_kv(name, found_str)
                .inspect_err(|e| errors.push((*e).clone()))
                .unwrap_or_default();

            match component_type {
                ComponentTypes::SelfClosing => {
                    let result = get_component_self(src, name, targets, hist.clone());
                    errors.extend(result.errors);
                    replacements.push((found_str.to_string(), result.output));
                }
                ComponentTypes::Wrapping => {
                    let end = format!("</{}>", name);
                    let slot_content = get_inside(output.clone(), found_str, &end);
                    let result =
                        get_component_slot(src, name, targets, slot_content.clone(), hist.clone());
                    errors.extend(result.errors);

                    if let Some(content) = slot_content {
                        replacements.push((content, String::new()));
                    }
                    replacements.push((end, String::new()));
                    replacements.push((found_str.to_string(), result.output));
                }
            }
        }
    }

    for (from, to) in replacements {
        output = output.replacen(&from, &to, 1);
    }

    ProcessResult { output, errors }
}
