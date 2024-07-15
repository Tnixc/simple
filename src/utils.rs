use crate::error::rewrite_error;
use crate::error::ErrorType;
use crate::error::PageHandleError;
use crate::error::WithItem;
use fancy_regex::Regex;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::str;
use std::{io::Error, path::PathBuf};
use ErrorType::Io;
use ErrorType::NotFound;
use ErrorType::Utf8;
use WithItem::Component;
use WithItem::Data;
use WithItem::File;
use WithItem::Template;

const COMPONENT_PATTERN_OPEN: &str =
    r#"(?<!<!--)<([A-Z][A-Za-z_]*(\/[A-Z][A-Za-z_]*)*)(\s+[a-z]+=(["'])[^"']*\4)*\s*>(?!.*?-->)"#;

// const _COMPONENT_PATTERN_: &str =
//     r"(?<!<!--)<([A-Z][A-Za-z_]*(\/[A-Z][A-Za-z_]*)*)\s*\/>(?!.*?-->)";

const TEMPLATE_PATTERN: &str =
    r"(?<!<!--)<-\{([A-Z][A-Za-z_]*(\/[A-Z][A-Za-z_]*)*)\}\s*\/>(?!.*?-->)";
// Thank you ChatGPT, couldn't have done this Regex-ing without you.

pub fn sub_component(src: &PathBuf, component: &str) -> Result<String, PageHandleError> {
    let path = src
        .join("components")
        .join(component)
        .with_extension("component.html");

    let contents = rewrite_error(fs::read(path.clone()), Component, NotFound, &path)?;

    return page(src, contents, false);
}

pub fn sub_template(src: &PathBuf, name: &str) -> Result<String, PageHandleError> {
    let template_path = src
        .join("templates")
        .join(name)
        .with_extension("template.html");
    let data_path = src.join("data").join(name).with_extension("data.json");

    let template_content_utf =
        rewrite_error(fs::read(&template_path), Template, NotFound, &template_path)?;

    let template = rewrite_error(
        String::from_utf8(template_content_utf),
        Template,
        Utf8,
        &template_path,
    )?;

    let data_content_utf8 = rewrite_error(fs::read(&data_path), Data, NotFound, &data_path)?;
    let data_str = str::from_utf8(&data_content_utf8).unwrap();
    let v: Value = serde_json::from_str(data_str).expect("JSON decode error");
    let items = v.as_array().expect("JSON wasn't an array");

    let mut contents = String::new();
    for object in items {
        let mut this = template.clone();
        for (key, value) in object.as_object().expect("Invalid object in JSON") {
            let key = format!("{{{}}}", key);
            this = this.replace(
                key.as_str(),
                value
                    .as_str()
                    .expect("JSON object value couldn't be decoded to string"),
            );
        }
        contents.push_str(&this);
    }

    return page(src, contents.into_bytes(), false);
}

fn page(src: &PathBuf, contents: Vec<u8>, dev: bool) -> Result<String, PageHandleError> {
    let mut string = rewrite_error(String::from_utf8(contents), File, Io, src)?;

    let re_component =
        Regex::new(COMPONENT_PATTERN_OPEN).expect("Regex failed to parse. This shouldn't happen.");
    for f in re_component.find_iter(&string.clone()) {
        if f.is_ok() {
            let found = f.unwrap();
            let trim = found
                .as_str()
                .trim()
                .trim_start_matches("<")
                .trim_end_matches(">")
                .trim();
            string = string.replace(
                found.as_str(),
                &sub_component(src, trim.split_once(" ").map(|(f, _)| f).unwrap_or(trim))?,
            );
            println!("Using: {:?}", found.as_str())
        }
    }

    let re_template =
        Regex::new(TEMPLATE_PATTERN).expect("Regex failed to parse. This shouldn't happen.");
    for f in re_template.find_iter(&string.clone()) {
        if f.is_ok() {
            let found = f.unwrap();
            string = string.replace(
                found.as_str(),
                &sub_template(
                    src,
                    found
                        .as_str()
                        .trim()
                        .trim_start_matches("<-{")
                        .trim_end_matches("/>")
                        .trim()
                        .trim_end_matches("}"),
                )?,
            );
            println!("Using: {:?}", found.as_str())
        }
    }

    if dev {
        string = string.replace(
            "<head>",
            "<head><script type='text/javascript' src='https://livejs.com/live.js'></script>",
        );
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
    let entries = rewrite_error(fs::read_dir(pages), File, Io, src)?;
    for entry in entries {
        if entry.as_ref().unwrap().path().is_dir() {
            let this = entry.unwrap().path();
            process_pages(&dir, &src, source.join(&this), this, dev)?;
        } else {
            let result = page(src, fs::read(entry.as_ref().unwrap().path()).unwrap(), dev);

            let path = dir.join("dist").join(
                entry
                    .unwrap()
                    .path()
                    .strip_prefix(src)
                    .unwrap()
                    .strip_prefix("pages")
                    .unwrap(),
            );
            rewrite_error(fs::create_dir_all(path.parent().unwrap()), File, Io, src)?;
            let mut f = rewrite_error(std::fs::File::create_new(&path), File, Io, src)?;
            println!("With: {:?}", path);
            rewrite_error(f.write(result?.as_bytes()), File, Io, src)?;
        }
    }
    Ok(())
}

pub fn copy_into(public: &PathBuf, dist: &PathBuf) -> Result<(), Error> {
    if !dist.exists() {
        fs::create_dir_all(dist)?;
    }

    let entries = fs::read_dir(public)?;
    for entry in entries {
        let entry = entry.unwrap().path();
        let dest_path = dist.join(entry.strip_prefix(public).unwrap());

        if entry.is_dir() {
            copy_into(&entry, &dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&entry, &dest_path)?;
        }
    }
    Ok(())
}
