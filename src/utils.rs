use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::fs::DirEntry;
use std::io;
use std::str;
use std::{io::Error, path::PathBuf};

pub fn sub_component(src: PathBuf, base: &str, component: &str) -> Result<String, Error> {
    let path = src.join(base).join(component).with_extension("html");

    println!("{:?}", path);
    let contents = fs::read(path)?;
    let string = String::from_utf8(contents).expect("Invalid UTF-8");
    return Ok(string);
}
fn page(entry: DirEntry) -> () {
    println!("{:?}", entry.path());
    let contents = fs::read(entry.path()).unwrap();
    let pattern = r"<([A-Z][A-Za-z_]*(/[A-Z][A-Za-z_]*)*)\s*/>"; // Thank you ChatGPT, couldn't have done this without you.
    let re = Regex::new(pattern).unwrap();
    for mat in re.find_iter(str::from_utf8(&contents).unwrap()) {
        println!("Found match: {}", mat.as_str());
    }
}

pub fn process_pages(source: PathBuf, pages: PathBuf) -> () {
    let entries = fs::read_dir(pages).unwrap();

    for entry in entries {
        if entry.as_ref().unwrap().path().is_dir() {
            let this = entry.unwrap().path();
            process_pages(source.join(&this), this);
        } else {
            page(entry.unwrap());
        }
    }
}
