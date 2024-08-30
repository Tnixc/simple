use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use crate::handlers::components::{process_component, ComponentTypes};
use crate::handlers::markdown::markdown_element;
use crate::handlers::templates::process_template;
use crate::utils::ProcessResult;
use color_print::cprintln;
use std::sync::mpsc;
use std::thread;
use std::{collections::HashSet, fs, io::Write, path::PathBuf};

const SCRIPT: &str = include_str!("../dev/inline_script.html");

fn process_step<F>(
    f: F,
    src: &PathBuf,
    string: &mut String,
    hist: &HashSet<PathBuf>,
    vec_errs: &mut Vec<ProcessError>,
) where
    F: Fn(&PathBuf, String, &HashSet<PathBuf>) -> ProcessResult,
{
    let result = f(src, string.clone(), hist);
    *string = result.output;
    vec_errs.extend(result.errors);
}

pub fn page(
    src: &PathBuf,
    mut string: String,
    dev: bool,
    hist: HashSet<PathBuf>,
) -> Result<String, ProcessError> {
    if string.contains("</markdown>") {
        string = markdown_element(string);
    }

    let mut vec_errs: Vec<ProcessError> = Vec::new();

    process_step(
        |srcpath, str, hist| {
            process_component(srcpath, str, ComponentTypes::Wrapping, hist.clone())
        },
        src,
        &mut string,
        &hist,
        &mut vec_errs,
    );
    process_step(
        |srcpath, str, hist| {
            process_component(srcpath, str, ComponentTypes::SelfClosing, hist.clone())
        },
        src,
        &mut string,
        &hist,
        &mut vec_errs,
    );
    process_step(
        |srcpath, str, hist| process_template(srcpath, str, hist.clone()),
        src,
        &mut string,
        &hist,
        &mut vec_errs,
    );

    if dev {
        string = string.replace("<head>", &format!("<head>{}", SCRIPT));
    }

    let mut e_i = 1;
    for e in vec_errs {
        cprintln!("<strong><r>Error {e_i}</></>: {e}");
        e_i += 1;
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
    let entries = fs::read_dir(pages).map_proc_err(WithItem::File, ErrorType::Io, src, None)?;
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
                            fs::read_to_string(&path).map_proc_err(
                                WithItem::File,
                                ErrorType::Io,
                                &path,
                                None,
                            )?,
                            dev,
                            HashSet::new(),
                        )?;
                        let out_path = dir.join(&s).join(
                            path.strip_prefix(&src)
                                .unwrap()
                                .strip_prefix("pages")
                                .unwrap(),
                        );

                        fs::create_dir_all(out_path.parent().unwrap()).map_proc_err(
                            WithItem::File,
                            ErrorType::Io,
                            &out_path,
                            None,
                        )?;
                        let mut f = std::fs::File::create(&out_path).map_proc_err(
                            WithItem::File,
                            ErrorType::Io,
                            &out_path,
                            None,
                        )?;
                        f.write_all(result.as_bytes()).map_proc_err(
                            WithItem::File,
                            ErrorType::Io,
                            &out_path,
                            None,
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
