use crate::error::ErrorType::Io;
use crate::error::{ErrorType, MapPageError, PageHandleError, WithItem};
use fancy_regex::Regex;
use std::fs;
use std::path::PathBuf;
use WithItem::File;

pub fn get_targets_kv<'a>(
    name: &str,
    found: &'a str,
) -> Result<Vec<(&'a str, &'a str)>, PageHandleError> {
    let mut targets: Vec<(&str, &str)> = Vec::new();
    // Regex for key-value pairs in components
    let re = Regex::new(r#"(\w+)=(['"])(?:(?!\2).)*\2"#).unwrap();
    let str = found
        .trim_start_matches(&("<".to_owned() + name))
        .trim_end_matches(">")
        .trim_end_matches("/>");

    for item in re.find_iter(str) {
        if let Ok(item) = item {
            if let Some((k, mut v)) = item.as_str().split_once('=') {
                v = v.trim_matches(|c| c == '\'' || c == '"');
                targets.push((k, v));
            } else {
                return Err(PageHandleError {
                    error_type: ErrorType::Syntax,
                    item: WithItem::Component,
                    path: PathBuf::from(name),
                });
            }
        } else {
            return Err(PageHandleError {
                error_type: ErrorType::Syntax,
                item: WithItem::Component,
                path: PathBuf::from(name),
            });
        }
    }
    Ok(targets)
}

pub fn kv_replace(kv: Vec<(&str, &str)>, mut from: String) -> String {
    for (k, v) in kv {
        let key = format!("${{{k}}}");
        from = from.replace(&key, v);
    }
    from
}

pub fn get_inside(input: &str, from: &str, to: &str) -> Option<String> {
    let start_index = input.find(from)?;
    let start_pos = start_index + from.len();
    let end_index = input[start_pos..].find(to).map(|i| i + start_pos)?;

    if start_pos >= end_index {
        None
    } else {
        Some(input[start_pos..end_index].to_string())
    }
}

pub fn copy_into(public: &PathBuf, dist: &PathBuf) -> Result<(), PageHandleError> {
    if !dist.exists() {
        fs::create_dir_all(dist).map_page_err(File, Io, &PathBuf::from(dist))?;
    }

    let entries = fs::read_dir(public).map_page_err(File, Io, &PathBuf::from(public))?;

    for entry in entries {
        let entry = entry.unwrap().path();
        let dest_path = dist.join(entry.strip_prefix(public).unwrap());

        if entry.is_dir() {
            copy_into(&entry, &dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_page_err(File, Io, &PathBuf::from(&dest_path))?;
            }
            fs::copy(&entry, &dest_path).map_page_err(File, Io, &PathBuf::from(&dest_path))?;
        }
    }
    Ok(())
}
