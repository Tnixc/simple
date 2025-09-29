use crate::error::ErrorType::Io;
use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use color_print::cformat;
use fancy_regex::Regex;
use once_cell::sync::Lazy;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use WithItem::File;

static KV_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\w+)=(['"])(?:(?!\2).)*\2"#)
        .expect("Regex failed to parse, this shouldn't happen")
});

pub fn get_targets_kv<'a>(
    name: &str,
    found: &'a str,
) -> Result<Vec<(&'a str, &'a str)>, ProcessError> {
    let mut targets = Vec::new();

    let start_tag = format!("<{}", name);
    let trimmed = found
        .strip_prefix(&start_tag)
        .unwrap_or(found)
        .trim_end_matches('>')
        .trim_end_matches("/>");

    for item in KV_REGEX.find_iter(trimmed) {
        if let Ok(item) = item {
            if let Some((k, mut v)) = item.as_str().split_once('=') {
                v = v.trim_matches(|c| c == '\'' || c == '"');
                targets.push((k, v));
            } else {
                return Err(ProcessError {
                    error_type: ErrorType::Syntax,
                    item: WithItem::Component,
                    path: PathBuf::from(name),
                    message: Some("Couldn't split key-value pair.".to_string()),
                });
            }
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

pub fn get_inside(input: String, from: &str, to: &str) -> Option<String> {
    let start_index = input.find(from)?;
    let start_pos = start_index + from.len();
    let end_index = input[start_pos..].find(to).map(|i| i + start_pos)?;

    if start_pos >= end_index {
        None
    } else {
        Some(input[start_pos..end_index].to_string())
    }
}

pub fn copy_into(public: &PathBuf, dist: &PathBuf) -> Result<(), ProcessError> {
    if !dist.exists() {
        fs::create_dir_all(dist).map_proc_err(File, Io, &PathBuf::from(dist), None)?;
    }

    let entries = fs::read_dir(public).map_proc_err(File, Io, &PathBuf::from(public), None)?;

    for entry in entries {
        let entry = entry.unwrap().path();
        let dest_path = dist.join(entry.strip_prefix(public).unwrap());

        if entry.is_dir() {
            copy_into(&entry, &dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_proc_err(
                    File,
                    Io,
                    &PathBuf::from(&dest_path),
                    None,
                )?;
            }
            fs::copy(&entry, &dest_path).map_proc_err(
                File,
                Io,
                &PathBuf::from(&dest_path),
                None,
            )?;
        }
    }
    Ok(())
}

pub fn unindent(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();

    if lines.is_empty() {
        return String::new();
    }

    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    if min_indent == 0 {
        return input.to_string();
    }

    let mut result = String::with_capacity(input.len());
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        if line.len() > min_indent && !line.trim().is_empty() {
            result.push_str(&line[min_indent..]);
        } else {
            result.push_str(line.trim_start());
        }
    }
    result
}

pub struct ProcessResult {
    pub output: String,
    pub errors: Vec<ProcessError>,
}

pub fn print_vec_errs(errors: &Vec<ProcessError>) {
    for (i, er) in errors.iter().enumerate() {
        eprintln!("{}", cformat!("<s><r>Build error {}</></>: {er}", i + 1));
    }
}

pub fn format_errs(errors: &Vec<ProcessError>) -> String {
    let mut msg = String::with_capacity(errors.len() * 100);
    for (i, er) in errors.iter().enumerate() {
        msg.push_str(&format!(
            "<p>{}</p>",
            cformat!("<s><r>Build error {}</></>: {er}\n", i + 1)
        ));
    }
    msg
}

pub fn walk_dir(dir: &PathBuf) -> Result<Vec<PathBuf>, ProcessError> {
    let mut files = Vec::new();
    walk_dir_internal(dir, &mut files)?;
    Ok(files)
}

fn walk_dir_internal(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), ProcessError> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_proc_err(
            WithItem::File,
            ErrorType::Io,
            &PathBuf::from(dir),
            None,
        )? {
            let entry =
                entry.map_proc_err(WithItem::File, ErrorType::Io, &PathBuf::from(dir), None)?;
            let path = entry.path();
            if path.is_dir() {
                walk_dir_internal(&path, files)?;
            } else {
                files.push(path);
            }
        }
    }
    Ok(())
}

pub fn find_next_available_port(start_port: u16) -> u16 {
    (start_port..65535)
        .find(|port| is_port_available(*port))
        .expect("No available ports found")
}

fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("0.0.0.0", port)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_targets_kv() {
        let name = "component";
        let found = r#"<component key1="value1" key2='value2'>"#;
        let result = get_targets_kv(name, found).unwrap();
        assert_eq!(result, vec![("key1", "value1"), ("key2", "value2")]);
    }

    #[test]
    fn test_kv_replace() {
        let kv = vec![("v1", "k1"), ("something", "else")];
        let from = "Hello, ${v1}! There is no key for something else.".to_string();
        let result = kv_replace(kv, from);
        assert_eq!(result, "Hello, k1! There is no key for something else.");
    }

    #[test]
    fn test_get_inside() {
        let input = "Hello {world} how are you?".to_string();
        let result = get_inside(input, "{", "}");
        assert_eq!(result, Some("world".to_string()));
    }

    #[test]
    fn test_unindent() {
        let input = "
            Hello
                World
                  How
                Are
            You
            ";
        let expected = "
Hello
    World
      How
    Are
You
";
        let result = unindent(input);
        assert_eq!(result, expected);
    }
}
