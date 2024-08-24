use crate::component_handler::process_component;
use crate::error::{ErrorType, MapPageError, PageHandleError, WithItem};
use crate::markdown::markdown_element;
use crate::template_handler::process_template;
use std::{collections::HashSet, fs, io::Write, path::PathBuf};

const SCRIPT: &str = include_str!("dev.html");

pub fn page(
    src: &PathBuf,
    contents: Vec<u8>,
    dev: bool,
    hist: HashSet<PathBuf>,
) -> Result<String, PageHandleError> {
    let mut string =
        String::from_utf8(contents).map_page_err(WithItem::File, ErrorType::Io, src)?;

    if string.contains("</markdown>") {
        string = markdown_element(string);
    }

    process_component(src, &mut string, "open", hist.clone())?;
    process_component(src, &mut string, "self", hist.clone())?;
    process_template(src, &mut string, hist.clone())?;

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
) -> Result<(), PageHandleError> {
    let entries = fs::read_dir(pages).map_page_err(WithItem::File, ErrorType::Io, src)?;
    let s = if dev { "dev" } else { "dist" };
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                process_pages(&dir, &src, source.join(&path), path, dev)?;
            } else {
                let result = page(
                    src,
                    fs::read(&path).map_page_err(WithItem::File, ErrorType::Io, &path)?,
                    dev,
                    HashSet::new(),
                )?;
                let out_path = dir.join(s).join(
                    path.strip_prefix(src)
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
            }
        }
    }
    Ok(())
}