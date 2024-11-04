use crate::dev::{SCRIPT, WS_PORT};
use crate::error::{ErrorType, MapProcErr, ProcessError, WithItem};
use crate::handlers::components::{process_component, ComponentTypes};
use crate::handlers::markdown::render_markdown;
use crate::handlers::templates::process_template;
use crate::utils::ProcessResult;
use crate::IS_DEV;
use minify_html::minify;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::{collections::HashSet, fs, io::Write, path::PathBuf};

fn process_step<F>(
    func: F,
    src: &PathBuf,
    string: &mut String,
    hist: &HashSet<PathBuf>,
    vec_errs: &mut Vec<ProcessError>,
) where
    F: Fn(&PathBuf, String, &HashSet<PathBuf>) -> ProcessResult,
{
    let result = func(src, string.to_string(), hist);
    *string = result.output;
    vec_errs.extend(result.errors);
}

pub fn page(src: &PathBuf, mut string: String, hist: HashSet<PathBuf>) -> ProcessResult {
    if string.contains("</markdown>") {
        string = render_markdown(string);
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
) -> Result<(), Vec<ProcessError>> {
    let mut errors: Vec<ProcessError> = Vec::new();
    let dev = *IS_DEV.get().unwrap();
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

    let minify_cfg = Arc::new(minify_html::Cfg::spec_compliant());

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                if let Err(mut errs) = process_pages(&dir, &src, source.join(&path), path) {
                    errors.append(&mut errs);
                }
            } else {
                let sender = sender.clone();
                let dir = dir.clone();
                let src = src.clone();
                let s = working_dir.to_string();

                let minify_cfg = Arc::clone(&minify_cfg);

                thread::spawn(move || {
                    let result = (|| -> Result<(), Vec<ProcessError>> {
                        let mut errors: Vec<ProcessError> = Vec::new();
                        let file_content = fs::read_to_string(&path)
                            .map_proc_err(WithItem::File, ErrorType::Io, &path, None)
                            .inspect_err(|e| errors.push((*e).clone()))
                            .unwrap_or(String::new());

                        let result = page(&src, file_content, HashSet::new());

                        let out_path = dir.join(&s).join(
                            path.strip_prefix(&src)
                                .unwrap()
                                .strip_prefix("pages")
                                .unwrap(),
                        );

                        let _ = fs::create_dir_all(out_path.parent().unwrap())
                            .map_proc_err(WithItem::File, ErrorType::Io, &out_path, None)
                            .inspect_err(|e| errors.push((*e).clone()));
                        let f = std::fs::File::create(&out_path)
                            .map_proc_err(WithItem::File, ErrorType::Io, &out_path, None)
                            .inspect_err(|e| errors.push((*e).clone()));
                        match f {
                            Ok(mut f) => {
                                let to_write = if dev {
                                    let ws_port = *WS_PORT.get().unwrap();
                                    let mut s = result.output;
                                    if !s.contains("// * SCRIPT INCLUDED IN DEV MODE") {
                                        s = s.replace("<head>", &format!("<head>{}", SCRIPT));
                                        s = s.replace(
                                            "__SIMPLE_WS_PORT_PLACEHOLDER__",
                                            ws_port.to_string().as_str(),
                                        );
                                    }
                                    s.as_bytes().to_vec()
                                } else {
                                    let mut w = result.output.as_bytes();
                                    let minified = minify(&mut w, &minify_cfg);
                                    minified
                                };

                                let _ = f
                                    .write_all(to_write.as_slice())
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
