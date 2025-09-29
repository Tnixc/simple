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
use std::{collections::HashSet, fs, path::PathBuf};

fn process_step<F>(
    func: F,
    src: &PathBuf,
    string: &mut String,
    hist: &HashSet<PathBuf>,
    vec_errs: &mut Vec<ProcessError>,
) where
    F: Fn(&PathBuf, String, &HashSet<PathBuf>) -> ProcessResult,
{
    let result = func(src, std::mem::take(string), hist);
    *string = result.output;
    vec_errs.extend(result.errors);
}

pub fn page(src: &PathBuf, mut string: String, hist: HashSet<PathBuf>) -> ProcessResult {
    let mut errors: Vec<ProcessError> = Vec::new();

    if string.contains("</markdown>") {
        string = render_markdown(string);
    }

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

    ProcessResult {
        output: string,
        errors,
    }
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

    let mut file_tasks = Vec::new();
    let mut dir_tasks = Vec::new();

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                dir_tasks.push((dir.clone(), src.clone(), source.join(&path), path));
            } else {
                file_tasks.push(path);
            }
        }
    }

    for (dir, src, source, path) in dir_tasks {
        if let Err(mut errs) = process_pages(&dir, &src, source, path) {
            errors.append(&mut errs);
        }
    }

    if !file_tasks.is_empty() {
        let max_threads = std::cmp::min(file_tasks.len(), num_cpus::get());
        let chunk_size = (file_tasks.len() + max_threads - 1) / max_threads;

        for chunk in file_tasks.chunks(chunk_size) {
            for path in chunk {
                let sender = sender.clone();
                let dir = dir.clone();
                let src = src.clone();
                let path = path.clone();
                let working_dir = working_dir.to_string();
                let minify_cfg = Arc::clone(&minify_cfg);

                thread::spawn(move || {
                    let result = process_single_file(path, dir, src, working_dir, dev, minify_cfg);
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

fn process_single_file(
    path: PathBuf,
    dir: PathBuf,
    src: PathBuf,
    working_dir: String,
    dev: bool,
    minify_cfg: Arc<minify_html::Cfg>,
) -> Result<(), Vec<ProcessError>> {
    let mut errors: Vec<ProcessError> = Vec::new();

    let file_content = fs::read_to_string(&path)
        .map_proc_err(WithItem::File, ErrorType::Io, &path, None)
        .inspect_err(|e| errors.push((*e).clone()))
        .unwrap_or_else(|_| String::new());

    if file_content.is_empty() && !errors.is_empty() {
        return Err(errors);
    }

    let result = page(&src, file_content, HashSet::new());
    errors.extend(result.errors);

    let out_path = dir.join(&working_dir).join(
        path.strip_prefix(&src)
            .unwrap()
            .strip_prefix("pages")
            .unwrap(),
    );

    if let Some(parent) = out_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: out_path.clone(),
                message: Some(format!("Failed to create directory: {}", e)),
            });
            return Err(errors);
        }
    }

    let mut output = result.output;

    if dev {
        let ws_port = *WS_PORT.get().unwrap();
        if !output.contains("// * SCRIPT INCLUDED IN DEV MODE") {
            output = output.replace("<head>", &format!("<head>{}", SCRIPT));
            output = output.replace("__SIMPLE_WS_PORT_PLACEHOLDER__", &ws_port.to_string());
        }
    }

    let to_write = if dev {
        output.into_bytes()
    } else {
        minify(output.as_bytes(), &minify_cfg)
    };

    match fs::write(&out_path, &to_write) {
        Ok(_) => Ok(()),
        Err(e) => {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: out_path,
                message: Some(format!("Failed to write file: {}", e)),
            });
            Err(errors)
        }
    }
}
