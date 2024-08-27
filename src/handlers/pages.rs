use crate::error::{ErrorType, MapPageError, ProcessError, WithItem};
use crate::handlers::components::{process_component, ComponentTypes};
use crate::handlers::markdown::markdown_element;
use crate::handlers::templates::process_template;
use std::sync::mpsc;
use std::thread;
use std::{collections::HashSet, fs, io::Write, path::PathBuf};

const SCRIPT: &str = include_str!("../dev/inline_script.html");

pub fn page(
    src: &PathBuf,
    contents: Vec<u8>,
    dev: bool,
    hist: HashSet<PathBuf>,
) -> Result<String, ProcessError> {
    let mut string =
        String::from_utf8(contents).map_page_err(WithItem::File, ErrorType::Io, src)?;

    if string.contains("</markdown>") {
        string = markdown_element(string);
    }

    string = process_component(src, string, ComponentTypes::Wrapping, hist.clone())?;
    string = process_component(src, string, ComponentTypes::SelfClosing, hist.clone())?;
    string = process_template(src, string, hist.clone())?;

    if dev {
        string = string.replace("<head>", &format!("<head>{}", SCRIPT));
    }

    Ok(string)
}

pub fn process_pages(
    dir: &PathBuf,
    src: &PathBuf,
    source: PathBuf,
    pages: PathBuf,
    dev: bool,
) -> Result<(), ProcessError> {
    let entries = fs::read_dir(pages).map_page_err(WithItem::File, ErrorType::Io, src)?;
    let working_dir = if dev { "dev" } else { "dist" };

    let (sender, receiver) = mpsc::channel();
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                process_pages(&dir, &src, source.join(&path), path, dev)?;
            } else {
                let sender = sender.clone();
                let dir = dir.clone();
                let src = src.clone();
                let s = working_dir.to_string();

                thread::spawn(move || {
                    let result = (|| -> Result<(), ProcessError> {
                        let result = page(
                            &src,
                            fs::read(&path).map_page_err(WithItem::File, ErrorType::Io, &path)?,
                            dev,
                            HashSet::new(),
                        )?;
                        let out_path = dir.join(&s).join(
                            path.strip_prefix(&src)
                                .unwrap()
                                .strip_prefix("pages")
                                .unwrap(),
                        );

                        fs::create_dir_all(out_path.parent().unwrap()).map_page_err(
                            WithItem::File,
                            ErrorType::Io,
                            &out_path,
                        )?;
                        let mut f = std::fs::File::create(&out_path).map_page_err(
                            WithItem::File,
                            ErrorType::Io,
                            &out_path,
                        )?;
                        f.write_all(result.as_bytes()).map_page_err(
                            WithItem::File,
                            ErrorType::Io,
                            &out_path,
                        )?;
                        Ok(())
                    })();

                    sender.send(result).unwrap();
                });
            }
        }
    }

    drop(sender);
    for result in receiver {
        result?;
    }
    Ok(())
}
