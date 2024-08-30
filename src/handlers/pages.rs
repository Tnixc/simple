use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use crate::handlers::components::{process_component, ComponentTypes};
use crate::handlers::markdown::markdown_element;
use crate::handlers::templates::process_template;
use crate::utils::ProcessResult;
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

pub fn page(src: &PathBuf, mut string: String, dev: bool, hist: HashSet<PathBuf>) -> ProcessResult {
    if string.contains("</markdown>") {
        string = markdown_element(string);
    }

    let mut errors: Vec<ProcessError> = Vec::new();

    process_step(
        |srcpath, str, hist| {
            process_component(srcpath, str, ComponentTypes::Wrapping, hist.clone())
        },
        src,
        &mut string,
        &hist,
        &mut errors,
    );
    process_step(
        |srcpath, str, hist| {
            process_component(srcpath, str, ComponentTypes::SelfClosing, hist.clone())
        },
        src,
        &mut string,
        &hist,
        &mut errors,
    );
    process_step(
        |srcpath, str, hist| process_template(srcpath, str, hist.clone()),
        src,
        &mut string,
        &hist,
        &mut errors,
    );

    if dev {
        string = string.replace("<head>", &format!("<head>{}", SCRIPT));
    }

    return ProcessResult {
        output: string,
        errors,
    };
}
pub fn process_pages(
    dir: &PathBuf,
    src: &PathBuf,
    source: PathBuf,
    pages: PathBuf,
    dev: bool,
) -> Result<(), Vec<ProcessError>> {
    let mut errors: Vec<ProcessError> = Vec::new();

    let entries = match fs::read_dir(&pages) {
        Ok(entries) => entries,
        Err(e) => {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: pages.clone(),
                message: Some(format!("Error reading pages directory: {:?}", e)),
            });
            return Err(errors);
        }
    };

    let working_dir = if dev { "dev" } else { "dist" };
    let (sender, receiver) = mpsc::channel();

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                if let Err(mut errs) = process_pages(&dir, &src, source.join(&path), path, dev) {
                    errors.append(&mut errs);
                }
            } else {
                let sender = sender.clone();
                let dir = dir.clone();
                let src = src.clone();
                let s = working_dir.to_string();

                thread::spawn(move || {
                    let result = (|| -> Result<(), Vec<ProcessError>> {
                        let mut errors: Vec<ProcessError> = Vec::new();
                        let file_content = fs::read_to_string(&path)
                            .map_proc_err(WithItem::File, ErrorType::Io, &path, None)
                            .inspect_err(|e| errors.push((*e).clone()))
                            .unwrap_or(String::new());

                        let result = page(&src, file_content, dev, HashSet::new());

                        let out_path = dir.join(&s).join(
                            path.strip_prefix(&src)
                                .unwrap()
                                .strip_prefix("pages")
                                .unwrap(),
                        );

                        fs::create_dir_all(out_path.parent().unwrap())
                            .map_proc_err(WithItem::File, ErrorType::Io, &out_path, None)
                            .inspect_err(|e| errors.push((*e).clone()));
                        let f = std::fs::File::create(&out_path)
                            .map_proc_err(WithItem::File, ErrorType::Io, &out_path, None)
                            .inspect_err(|e| errors.push((*e).clone()));
                        match f {
                            Ok(mut f) => {
                                f.write_all(result.output.as_bytes())
                                    .map_proc_err(WithItem::File, ErrorType::Io, &out_path, None)
                                    .inspect_err(|e| errors.push((*e).clone()));
                            }
                            Err(_) => (),
                        }

                        if !result.errors.is_empty() {
                            return Err(result.errors);
                        }

                        Ok(())
                    })();

                    sender.send(result).unwrap();
                });
            }
        }
    }

    drop(sender);
    for result in receiver {
        if let Err(e) = result {
            errors.extend(e);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
