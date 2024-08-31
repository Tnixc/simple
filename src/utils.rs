use crate::error::ErrorType::Io;
use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use color_print::cformat;
use fancy_regex::Regex;
use lazy_static::lazy_static;
use std::fs;
use std::path::PathBuf;
use WithItem::File;

const KV_PATTERN: &str = r#"(\w+)=(['"])(?:(?!\2).)*\2"#;
lazy_static! {
    static ref KV_REGEX: Regex =
        Regex::new(KV_PATTERN).expect("Regex failed to parse, this shouldn't happen");
}

pub fn get_targets_kv<'a>(
    name: &str,
    found: &'a str,
) -> Result<Vec<(&'a str, &'a str)>, ProcessError> {
    let mut targets: Vec<(&str, &str)> = Vec::new();
    // Regex for key-value pairs in components
    let str = found
        .trim_start_matches(&("<".to_owned() + name))
        .trim_end_matches(">")
        .trim_end_matches("/>");

    for item in KV_REGEX.find_iter(str) {
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
        } else {
            return Err(ProcessError {
                error_type: ErrorType::Syntax,
                item: WithItem::Component,
                path: PathBuf::from(name),
                message: Some("Couldn't find key-value pair.".to_string()),
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

    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .into_iter()
        .map(|line| {
            if line.len() > min_indent {
                &line[min_indent..]
            } else {
                line.trim_start()
            }
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

pub struct ProcessResult {
    pub output: String,
    pub errors: Vec<ProcessError>,
}

pub fn print_vec_errs(errors: &Vec<ProcessError>) {
    let mut e_i = 1;
    for er in errors {
        eprintln!("{}", cformat!("<s><r>Build error {e_i}</></>: {er}"));
        e_i += 1;
    }
}

pub fn format_errs(errors: &Vec<ProcessError>) -> String {
    let mut e_i = 1;
    let mut msg = String::new();
    for er in errors {
        msg.push_str(&format!(
            "<p>{}</p>",
            cformat!("<s><r>Build error {e_i}</></>: {er}\n")
        ));
        e_i += 1;
    }
    msg
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
