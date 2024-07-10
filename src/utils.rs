use regex::Regex;
use std::fs;
use std::fs::DirEntry;
use std::fs::File;
use std::io::Write;
use std::str;
use std::{io::Error, path::PathBuf};

const PATTERN: &str = r"<([A-Z][A-Za-z_]*(/[A-Z][A-Za-z_]*)*)\s*/>";
// Thank you ChatGPT, couldn't have done this without you.

pub fn sub_component(src: &PathBuf, base: &str, component: &str) -> Result<String, Error> {
    let path = src.join(base).join(component).with_extension("html");

    println!("SUB COMPONENT: {:?}", path);
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
    println!("PAGE CALL: {:?}", entry.path());
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
    return Ok(string);
}

pub fn process_pages(
    dir: &PathBuf,
    src: &PathBuf,
    source: PathBuf,
    pages: PathBuf,
) -> Result<(), Error> {
    let entries = fs::read_dir(pages).unwrap();
    for entry in entries {
        if entry.as_ref().unwrap().path().is_dir() {
            let this = entry.unwrap().path();
            process_pages(&dir, &src, source.join(&this), this)?;
        } else {
            let result = page(src, entry.as_ref().unwrap());
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
            println!("{:?}", path);
            f.write(result.unwrap().as_bytes())?;
        }
    }
    Ok(())
}
