use regex::Regex;
use serde_json::Value;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::str;
use std::{io::Error, path::PathBuf};

use crate::error::PageHandleError;

const COMPONENT_PATTERN: &str = r"<([A-Z][A-Za-z_]*(\/[A-Z][A-Za-z_]*)*)\s*\/>";
const TEMPLATE_PATTERN: &str = r"<-\{([A-Z][A-Za-z_]*(\/[A-Z][A-Za-z_]*)*)\}\s*\/>";

// Thank you ChatGPT, couldn't have done this without you.

pub fn sub_component(src: &PathBuf, component: &str) -> Result<String, PageHandleError> {
    let path = src
        .join("components")
        .join(component)
        .with_extension(".component.html");

    let contents = fs::read(path.clone()).map_err(|e| PageHandleError::Io {
        source: e,
        item: path.into_os_string().into_string().unwrap(),
    })?;
    return page(src, contents);
}

pub fn sub_template(src: &PathBuf, name: &str) -> Result<String, PageHandleError> {
    let template_path = src
        .join("templates")
        .join(name)
        .with_extension(".template.html");
    let data_path = src.join("data").join(name).with_extension(".data.json");

    let template = String::from_utf8(fs::read(&template_path)?)?;
    let data = fs::read(&data_path)?;

    let data_str = str::from_utf8(&data).unwrap();
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

    return page(src, contents.into_bytes());
}

fn page(src: &PathBuf, contents: Vec<u8>) -> Result<String, PageHandleError> {
    let mut string = String::from_utf8(contents)?;

    let re_component = Regex::new(COMPONENT_PATTERN).unwrap();
    for found in re_component.find_iter(&string.clone()) {
        string = string.replace(
            found.as_str(),
            &sub_component(
                src,
                found
                    .as_str()
                    .trim_start_matches("<")
                    .trim_end_matches("/>")
                    .trim(),
            )?,
        );
        println!("Using: {:?}", found.as_str())
    }

    let re_template = Regex::new(TEMPLATE_PATTERN).unwrap();
    for found in re_template.find_iter(&string.clone()) {
        string = string.replace(
            found.as_str(),
            &sub_template(
                src,
                found
                    .as_str()
                    .trim_start_matches("<-{")
                    .trim_end_matches("/>")
                    .trim()
                    .trim_end_matches("}"),
            )?,
        );
        println!("Using: {:?}", found.as_str())
    }
    return Ok(string);
}

pub fn process_pages(
    dir: &PathBuf,
    src: &PathBuf,
    source: PathBuf,
    pages: PathBuf,
) -> Result<(), PageHandleError> {
    let entries = fs::read_dir(pages)?;
    for entry in entries {
        if entry.as_ref().unwrap().path().is_dir() {
            let this = entry.unwrap().path();
            process_pages(&dir, &src, source.join(&this), this)?;
        } else {
            let result = page(src, fs::read(entry.as_ref().unwrap().path()).unwrap());

            let path = dir.join("dist").join(
                entry
                    .unwrap()
                    .path()
                    .strip_prefix(src)
                    .unwrap()
                    .strip_prefix("pages")
                    .unwrap(),
            );
            fs::create_dir_all(path.parent().unwrap())?;
            let mut f = File::create_new(&path)?;
            println!("Writing - {:?}", path);
            f.write(result?.as_bytes())?;
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
