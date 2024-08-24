use crate::error::{ErrorType, MapPageError, PageHandleError, WithItem};
use crate::page_processor::page;
use crate::utils::{get_inside, kv_replace, targets_kv};
use color_print::cprintln;
use fancy_regex::Regex;
use std::{collections::HashSet, fs, path::PathBuf};

const COMPONENT_PATTERN_SELF: &str =
    r#"(?<!<!--)<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['\"]).*?\4)*\s*\/>(?!.*?-->)"#;

const COMPONENT_PATTERN_OPEN: &str =
    r#"(?<!<!--)<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['\"]).*?\4)*\s*>(?!.*?-->)"#;

const SLOT_PATTERN: &str = r#"(?<!<!--)<slot([\S\s])*>*?<\/slot>(?!.*?-->)"#;

pub fn get_component_self(
    src: &PathBuf,
    component: &str,
    targets: Vec<(&str, &str)>,
    mut hist: HashSet<PathBuf>,
) -> Result<String, PageHandleError> {
    let path = src
        .join("components")
        .join(component.replace(":", "/"))
        .with_extension("component.html");

    let v = fs::read(&path).map_page_err(WithItem::Component, ErrorType::NotFound, &path)?;
    let mut st = String::from_utf8(v).map_page_err(WithItem::Component, ErrorType::Utf8, &path)?;
    st = kv_replace(targets, st);
    let contents = st.clone().into_bytes();
    if !hist.insert(path.clone()) {
        return Err(PageHandleError {
            error_type: ErrorType::Circular,
            item: WithItem::Component,
            path: PathBuf::from(path),
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
) -> Result<String, PageHandleError> {
    let path = src
        .join("components")
        .join(component.replace(":", "/"))
        .with_extension("component.html");
    let v = fs::read(&path).map_page_err(WithItem::Component, ErrorType::NotFound, &path)?;
    let mut st = String::from_utf8(v).expect("Contents of component is not UTF8");

    if !st.contains("<slot>") || !st.contains("</slot>") {
        cprintln!("<r>Component does not contain a slot tag.</r>");
        return Err(PageHandleError {
            error_type: ErrorType::Syntax,
            item: WithItem::Component,
            path: PathBuf::from(component),
        });
    }

    st = kv_replace(targets, st);
    if let Some(content) = slot_content {
        let re = Regex::new(SLOT_PATTERN).expect("Failed to parse regex");
        // here it replaces "<slot>fallback</slot>" with "<slot></slot>, after the content is exists"
        st = re.replace(&st, &content).to_string();
    }

    if !hist.insert(path.clone()) {
        return Err(PageHandleError {
            error_type: ErrorType::Circular,
            item: WithItem::Component,
            path: PathBuf::from(path),
        });
    }

    return page(src, st.into_bytes(), false, hist);
}

pub fn process_component(
    src: &PathBuf,
    string: &mut String,
    component_type: &str,
    hist: HashSet<PathBuf>,
) -> Result<(), PageHandleError> {
    let pattern = match component_type {
        "self" => COMPONENT_PATTERN_SELF,
        "open" => COMPONENT_PATTERN_OPEN,
        _ => {
            return Err(PageHandleError {
                error_type: ErrorType::Syntax,
                item: WithItem::Component,
                path: PathBuf::from("unknown"),
            })
        }
    };

    let re = Regex::new(pattern).expect("Regex failed to parse. This shouldn't happen.");

    for f in re.find_iter(&string.to_owned()) {
        if let Ok(found) = f {
            let trim = found
                .as_str()
                .trim()
                .trim_start_matches("<")
                .trim_end_matches("/>")
                .trim_end_matches(">")
                .trim();
            let name = trim.split_whitespace().next().unwrap_or(trim);
            let targets = targets_kv(name, found.as_str())?;

            if component_type == "self" {
                let target = found.as_str();
                let replacement = get_component_self(src, name, targets, hist.clone())?;
                *string = string.replacen(target, &replacement, 1);
            } else {
                let end = format!("</{}>", &name);
                let slot_content = get_inside(string, found.as_str(), &end);
                let replacement =
                    get_component_slot(src, name, targets, slot_content.clone(), hist.clone())?;
                *string = string.replacen(slot_content.unwrap_or("".to_string()).as_str(), "", 1);
                *string = string.replacen(&end, "", 1);
                *string = string.replacen(found.as_str(), &replacement, 1);
            };
        }
    }
    Ok(())
}
