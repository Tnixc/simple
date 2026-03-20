use crate::dev::{SCRIPT, WS_PORT};
use crate::error::{errors_to_html, ErrorType, MapProcErr, ProcessError, WithItem};
use crate::handlers::components::{process_component, ComponentTypes};
use crate::handlers::katex_assets;
use crate::handlers::markdown::render_markdown;
use crate::handlers::templates::process_template;
use crate::utils::ProcessResult;
use crate::IS_DEV;
use minify_html::minify;
use rayon::prelude::*;
use std::sync::Arc;
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
        let md_result = render_markdown(string);
        string = md_result.output;
        errors.extend(md_result.errors);
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
    let minify_cfg = Arc::new(minify_html::Cfg::new());

    let mut file_tasks = Vec::new();
    let mut dir_tasks = Vec::new();

    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_dir() {
                    dir_tasks.push((dir.clone(), src.clone(), source.join(&path), path));
                } else {
                    file_tasks.push(path);
                }
            }
            Err(e) => {
                errors.push(ProcessError {
                    error_type: ErrorType::Io,
                    item: WithItem::File,
                    path: pages.clone(),
                    message: Some(format!("Failed to read directory entry: {}", e)),
                });
            }
        }
    }

    // Process directories sequentially
    for (dir, src, source, path) in dir_tasks {
        if let Err(mut errs) = process_pages(&dir, &src, source, path) {
            errors.append(&mut errs);
        }
    }

    // Process files in parallel using rayon
    if !file_tasks.is_empty() {
        let results: Vec<(Vec<ProcessError>,)> = file_tasks
            .par_iter()
            .map(|path| {
                let errs = process_single_file(
                    path.clone(),
                    dir.clone(),
                    src.clone(),
                    working_dir.to_string(),
                    dev,
                    Arc::clone(&minify_cfg),
                );
                (errs,)
            })
            .collect();

        for (errs,) in results {
            errors.extend(errs);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Process a single page file. Always returns collected errors (may be empty).
/// In dev mode, writes an error page if there are errors.
/// In build mode, skips writing the file if there are errors.
fn process_single_file(
    path: PathBuf,
    dir: PathBuf,
    src: PathBuf,
    working_dir: String,
    dev: bool,
    minify_cfg: Arc<minify_html::Cfg>,
) -> Vec<ProcessError> {
    let mut errors: Vec<ProcessError> = Vec::new();

    // Reset KaTeX usage flag for this page
    katex_assets::reset_katex_flag();

    let file_content =
        match fs::read_to_string(&path).map_proc_err(WithItem::File, ErrorType::Io, &path, None) {
            Ok(content) => content,
            Err(e) => {
                errors.push(e);
                write_error_page_if_dev(dev, &errors, &dir, &src, &path, &working_dir);
                return errors;
            }
        };

    if file_content.is_empty() {
        errors.push(ProcessError {
            error_type: ErrorType::Other,
            item: WithItem::File,
            path: path.clone(),
            message: Some("Page file is empty".to_string()),
        });
        write_error_page_if_dev(dev, &errors, &dir, &src, &path, &working_dir);
        return errors;
    }

    let result = page(&src, file_content, HashSet::new());
    errors.extend(result.errors);

    let out_path = match resolve_out_path(&path, &dir, &src, &working_dir) {
        Ok(p) => p,
        Err(e) => {
            errors.push(e);
            return errors;
        }
    };

    if let Some(parent) = out_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: out_path.clone(),
                message: Some(format!("Failed to create directory: {}", e)),
            });
            return errors;
        }
    }

    // If there are errors: dev → error page, build → skip
    if !errors.is_empty() {
        if dev {
            let dev_script = make_dev_script();
            let error_html = errors_to_html(&errors, dev_script.as_deref());
            let _ = fs::write(&out_path, error_html.as_bytes());
        }
        return errors;
    }

    let mut output = result.output;

    // Inject KaTeX CSS if math was rendered (unless disabled)
    if katex_assets::was_katex_used() && !katex_assets::is_katex_injection_disabled() {
        katex_assets::print_katex_message();

        if output.contains("<head>") {
            output = output.replace(
                "<head>",
                &format!("<head>\n{}", katex_assets::get_katex_css_tag()),
            );
        } else {
            output = format!("{}\n{}", katex_assets::get_katex_css_tag(), output);
        }
    }

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

    if let Err(e) = fs::write(&out_path, &to_write) {
        errors.push(ProcessError {
            error_type: ErrorType::Io,
            item: WithItem::File,
            path: out_path,
            message: Some(format!("Failed to write file: {}", e)),
        });
    }

    errors
}

fn resolve_out_path(
    path: &PathBuf,
    dir: &PathBuf,
    src: &PathBuf,
    working_dir: &str,
) -> Result<PathBuf, ProcessError> {
    let relative_to_src = path.strip_prefix(src).map_err(|e| ProcessError {
        error_type: ErrorType::Io,
        item: WithItem::File,
        path: path.clone(),
        message: Some(format!("Failed to strip src prefix: {}", e)),
    })?;

    let relative_to_pages = relative_to_src
        .strip_prefix("pages")
        .map_err(|e| ProcessError {
            error_type: ErrorType::Io,
            item: WithItem::File,
            path: path.clone(),
            message: Some(format!("Failed to strip pages prefix: {}", e)),
        })?;

    Ok(dir.join(working_dir).join(relative_to_pages))
}

fn make_dev_script() -> Option<String> {
    let ws_port = WS_PORT.get()?;
    Some(SCRIPT.replace("__SIMPLE_WS_PORT_PLACEHOLDER__", &ws_port.to_string()))
}

/// In dev mode, attempt to write an error page for the given source path.
/// Best-effort — if path resolution fails, this is a no-op.
fn write_error_page_if_dev(
    dev: bool,
    errors: &[ProcessError],
    dir: &PathBuf,
    src: &PathBuf,
    path: &PathBuf,
    working_dir: &str,
) {
    if !dev {
        return;
    }
    if let Ok(out_path) = resolve_out_path(path, dir, src, working_dir) {
        if let Some(parent) = out_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let dev_script = make_dev_script();
        let error_html = errors_to_html(errors, dev_script.as_deref());
        let _ = fs::write(&out_path, error_html.as_bytes());
    }
}
