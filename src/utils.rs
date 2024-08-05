use crate::dev::SCRIPT;
use std::collections::HashSet;
use crate::error::MapPageError;
use crate::error::{ErrorType, PageHandleError, WithItem};
use crate::markdown::markdown_element;
use fancy_regex::Regex;
use serde_json::Value;
use std::{fs, io::Write, path::PathBuf, str};
use ErrorType::{Io, NotFound, Syntax, Utf8, Circular};
use WithItem::{Component, Data, File, Template};

const COMPONENT_PATTERN_OPEN: &str =
    r#"(?<!<!--)<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['\"]).*?\4)*\s*>(?!.*?-->)"#;

const COMPONENT_PATTERN_SELF: &str =
    r#"(?<!<!--)<([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)(\s+[A-Za-z]+=(['\"]).*?\4)*\s*\/>(?!.*?-->)"#;

const TEMPLATE_PATTERN: &str =
    r#"(?<!<!--)<-Template\{([A-Z][A-Za-z_]*(:[A-Z][A-Za-z_]*)*)\}\s*\/>(?!.*?-->)"#;

const SLOT_PATTERN: &str = r#"(?<!<!--)<slot([\S\s])*>*?<\/slot>(?!.*?-->)"#;

const CLASS_PATTERN: &str = r#"(\w+)=(['"])(?:(?!\2).)*\2"#;
// Thank you Claude, couldn't have done this Regex-ing without you.

pub fn sub_component_self(
    src: &PathBuf,
    component: &str,
    targets: Vec<(&str, &str)>,
    mut hist: HashSet<PathBuf>,
) -> Result<String, PageHandleError> {
    let path = src
        .join("components")
        .join(component.replace(":", "/"))
        .with_extension("component.html");

    let v = fs::read(&path).map_page_err(Component, NotFound, &path)?;
    let mut st = String::from_utf8(v).map_page_err(Component, Utf8, &path)?;
    st = kv_replace(targets, st);
    let contents = st.clone().into_bytes();
    if !hist.insert(path.clone()) {
        return Err(PageHandleError { error_type: Circular, item: Component, path: PathBuf::from(path) })
    }
    return page(src, contents, false, hist);
}

pub fn sub_component_slot(
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
    let v = fs::read(&path).map_page_err(Component, NotFound, &path)?;
    let mut st = String::from_utf8(v).expect("Contents of component is not UTF8");

    if !st.contains("<slot>") || !st.contains("</slot>") {
        return Err(PageHandleError {
            error_type: Syntax,
            item: Component,
            path: PathBuf::from(component),
        });
    }

    st = kv_replace(targets, st);
    if slot_content.is_some() {
        let re = Regex::new(SLOT_PATTERN).expect("Failed to parse regex");
        st = re.replace(&st, "<slot></slot>").to_string();
        st = st.replace("</slot>", &(slot_content.unwrap() + "</slot>"));
    }
    if !hist.insert(path.clone()) {
        return Err(PageHandleError { error_type: Circular, item: Component, path: PathBuf::from(path) })
    }

    return page(src, st.into_bytes(), false, hist);
}

pub fn sub_template(
    src: &PathBuf,
    name: &str,
    mut hist: HashSet<PathBuf>,
) -> Result<String, PageHandleError> {
    let template_path = src
        .join("templates")
        .join(name.replace(":", "/"))
        .with_extension("template.html");

    let data_path = src
        .join("data")
        .join(name.replace(":", "/"))
        .with_extension("data.json");

    let template_content_utf =
        fs::read(&template_path).map_page_err(Template, NotFound, &template_path)?;
    let template =
        String::from_utf8(template_content_utf).map_page_err(Template, Utf8, &template_path)?;

    let data_content_utf8 = fs::read(&data_path).map_page_err(Data, NotFound, &data_path)?;
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
        return Err(PageHandleError { error_type: Circular, item: Template, path: template_path })
    }
    return page(src, contents.into_bytes(), false, hist);
}

fn page(
    src: &PathBuf,
    contents: Vec<u8>,
    dev: bool,
    hist: HashSet<PathBuf>,
) -> Result<String, PageHandleError> {

    let mut string = String::from_utf8(contents).map_page_err(File, Io, src)?;

    if string.contains("</markdown>") {
        string = markdown_element(string);
    }
    let re_component_self =
        Regex::new(COMPONENT_PATTERN_SELF).expect("Regex failed to parse. This shouldn't happen.");

    for f in re_component_self.find_iter(&string.to_owned()) {
        if f.is_ok() {
            let found = f.unwrap();
            let trim = found
                .as_str()
                .trim()
                .trim_start_matches("<")
                .trim_end_matches("/>")
                .trim();
            let name = trim.split_whitespace().next().unwrap_or(trim);
            let targets = targets_kv(name, found.as_str())?;
            string = string.replacen(
                found.as_str(),
                &sub_component_self(src, name, targets, hist.clone())?,
                1,
            );
        }
    }

    let re_component_open =
        Regex::new(COMPONENT_PATTERN_OPEN).expect("Regex failed to parse. This shouldn't happen.");

    for f in re_component_open.find_iter(&string.to_owned()) {
        if f.is_ok() {
            let found = f.unwrap();
            let trim = found
                .as_str()
                .trim()
                .trim_start_matches("<")
                .trim_end_matches(">")
                .trim();
            let name = trim.split_whitespace().next().unwrap_or(trim);
            let end = format!("</{}>", &name);

            let targets = targets_kv(name, found.as_str())?;
            let slot_content = get_inside(&string, found.as_str(), &end);
            if slot_content.is_some() {
                let from = found.as_str().to_owned() + &(slot_content.as_ref().unwrap().clone());
                string = string.replacen(
                    &from,
                    &sub_component_slot(src, name, targets, slot_content, hist.clone())?,
                    1,
                );
            } else {
                string = string.replacen(
                    &found.as_str().to_owned(),
                    &sub_component_slot(src, name, targets, slot_content, hist.clone())?,
                    1,
                )
            }

            string = string.replacen(&end, "", 1);
        }
    }

    let re_template =
        Regex::new(TEMPLATE_PATTERN).expect("Regex failed to parse. This shouldn't happen.");
    for f in re_template.find_iter(&string.clone()) {
        if f.is_ok() {
            let found = f.unwrap();
            string = string.replacen(
                found.as_str(),
                &sub_template(
                    src,
                    found
                        .as_str()
                        .trim()
                        .trim_start_matches("<-Template{")
                        .trim_end_matches("/>")
                        .trim()
                        .trim_end_matches("}"),
                    hist.clone()
                )?,
                1,
            );
        }
    }

    if dev {
        string = string.replace("<head>", ("<head>".to_owned() + SCRIPT).as_str());
    }
    return Ok(string);
}

pub fn process_pages(
    dir: &PathBuf,
    src: &PathBuf,
    source: PathBuf,
    pages: PathBuf,
    dev: bool,
) -> Result<(), PageHandleError> {
    // dir is the root.
    let entries = fs::read_dir(pages).map_page_err(File, Io, src)?;
    let s;
    if dev {
        s = "dev"
    } else {
        s = "dist"
    }
    for entry in entries {
        if entry.as_ref().unwrap().path().is_dir() {
            let this = entry.unwrap().path();
            process_pages(&dir, &src, source.join(&this), this, dev)?;
        } else {
            let result = page(
                src,
                fs::read(entry.as_ref().unwrap().path()).unwrap(),
                dev,
                HashSet::new(),
            );
            let path = dir.join(s).join(
                entry
                    .unwrap()
                    .path()
                    .strip_prefix(src)
                    .unwrap()
                    .strip_prefix("pages")
                    .unwrap(),
            );

            fs::create_dir_all(path.parent().unwrap()).map_page_err(File, Io, &path)?;
            let mut f = std::fs::File::create(&path).map_page_err(File, Io, &path)?;
            f.write(result?.as_bytes()).map_page_err(File, Io, &path)?;
        }
    }
    Ok(())
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

fn targets_kv<'a>(name: &str, found: &'a str) -> Result<Vec<(&'a str, &'a str)>, PageHandleError> {
    let mut targets: Vec<(&str, &str)> = Vec::new();
    let re = Regex::new(CLASS_PATTERN).unwrap();
    let str = found
        .trim_start_matches(&("<".to_owned() + name))
        .trim_end_matches(">");

    for item in re.find_iter(str) {
        if item.is_ok() {
            match item.unwrap().as_str().split_once("=") {
                Some((k, mut v)) => {
                    if v.starts_with("'") {
                        v = v.trim_start_matches("'").trim_end_matches("'");
                    } else if v.starts_with("\"") {
                        v = v.trim_start_matches("\"").trim_end_matches("\"");
                    }
                    targets.push((k, v))
                }
                None => {
                    eprintln!("Equals not found when parsing props.");
                    return Err(PageHandleError {
                        error_type: Syntax,
                        item: Component,
                        path: PathBuf::from(name),
                    });
                }
            }
        } else {
            eprintln!("Equals not found when parsing props.");
            return Err(PageHandleError {
                error_type: Syntax,
                item: Component,
                path: PathBuf::from(name),
            });
        }
    }
    return Ok(targets);
}

fn kv_replace(kv: Vec<(&str, &str)>, mut from: String) -> String {
    for (k, v) in kv {
        let key = format!("${{{k}}}");
        from = from.replace(&key, v);
    }
    return from;
}

fn get_inside(input: &str, from: &str, to: &str) -> Option<String> {
    let start_index = input.find(from)?;
    let start_pos = start_index + from.len();
    let end_index = input[start_pos..].find(to).map(|i| i + start_pos)?;

    if start_pos >= end_index {
        None
    } else {
        Some(input[start_pos..end_index].to_string())
    }
}
