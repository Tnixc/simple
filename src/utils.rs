use std::collections::HashMap;
use std::fs;
use std::io;
use std::{io::Error, path::PathBuf};

pub fn sub_component(src: PathBuf, component: &str) -> Result<String, Error> {
    let path = src
        .join("components")
        .join(component)
        .with_extension("html");

    println!("{:?}", path);
    let contents = fs::read(path)?;
    let string = String::from_utf8(contents).expect("Invalid UTF-8");
    return Ok(string);
}

pub fn process_pages(source: PathBuf, pages: PathBuf) -> () {
    let pages = fs::read_dir(source.join(pages)).unwrap();
    let mut store: HashMap<String, String> = HashMap::new();

    for entry in pages {
        if entry.as_ref().unwrap().path().is_dir() {
            let this = entry.unwrap().path();
            process_pages(source.join(this), this.clone());
        }
    }
}
