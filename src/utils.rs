use regex::Regex;
use std::fs;
use std::fs::DirEntry;
use std::str;
use std::{io::Error, path::PathBuf};

const PATTERN: &str = r"<([A-Z][A-Za-z_]*(/[A-Z][A-Za-z_]*)*)\s*/>";
// Thank you ChatGPT, couldn't have done this without you.

pub fn sub_component(src: &PathBuf, base: &str, component: &str) -> Result<String, Error> {
    let path = src.join(base).join(component).with_extension("html");

    println!("{:?}", path);
    let contents = fs::read(path)?;
    let mut string = String::from_utf8(contents).expect("Invalid UTF-8");

    let re = Regex::new(PATTERN).unwrap();

    for found in re.find_iter(&string.clone()) {
        string = string.replace(
            found.as_str(),
            &sub_component(
                src,
                "components",
                found.as_str().replace("<", "").replace("/>", "").trim(),
            )
            .unwrap(),
        );
        println!("Found: {:?}", found.as_str())
    }
    return Ok(string);
}

fn page(src: &PathBuf, entry: &DirEntry) -> Result<String, Error> {
    println!("{:?}", entry.path());
    let contents = fs::read(entry.path()).unwrap();
    let mut string = String::from_utf8(contents).expect("Invalid UTF-8");

    let re = Regex::new(PATTERN).unwrap();
    for found in re.find_iter(&string.clone()) {
        string = string.replace(
            found.as_str(),
            &sub_component(
                src,
                "components",
                found.as_str().replace("<", "").replace("/>", "").trim(),
            )
            .unwrap(),
        );
        println!("Found: {:?}", found.as_str())
    }
    println!("{:?}", string);
    return Ok(string);
}

pub fn process_pages(src: &PathBuf, source: PathBuf, pages: PathBuf) -> Result<(), Error> {
    let entries = fs::read_dir(pages).unwrap();

    for entry in entries {
        if entry.as_ref().unwrap().path().is_dir() {
            let this = entry.unwrap().path();
            process_pages(&src, source.join(&this), this)?;
        } else {
            let result = page(src, entry.as_ref().unwrap());
            fs::write(
                src.parent()
                    .unwrap()
                    .join("dist")
                    .join(entry.unwrap().path()),
                result.unwrap(),
            )?;
        }
    }

    Ok(())
}
