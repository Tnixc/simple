use crate::dev::{SCRIPT, WS_PORT};
use crate::error::{errors_to_html, ErrorType, ProcessError, WithItem};
use crate::handlers::frontmatter::extract_frontmatter;
use crate::handlers::katex_assets;
use crate::handlers::pages::page;
use crate::utils::kv_replace;
use crate::IS_DEV;
use minify_html::minify;
use std::{collections::HashSet, fs, path::PathBuf};

pub fn process_entry(
    src: &PathBuf,
    name: &str,
    entry_path: String,
    result_path: String,
    kv: Vec<(&str, &str)>,
) -> Vec<ProcessError> {
    let mut errors: Vec<ProcessError> = Vec::new();
    let is_dev = *IS_DEV.get().unwrap_or(&false);

    // Reset KaTeX usage flag for this page
    katex_assets::reset_katex_flag();

    if entry_path.is_empty() || result_path.is_empty() {
        return vec![ProcessError {
            error_type: ErrorType::Other,
            item: WithItem::Template,
            path: PathBuf::from(&result_path),
            message: Some(
                format!("Error occurred in {name}. The --entry-path and --result-path keys must both be present if either is present.")
            ),
        }];
    }

    let entry_path = src.join("data").join(entry_path.trim_start_matches("/"));
    let frame_path = src
        .join("templates")
        .join(name.replace(":", "/"))
        .with_extension("frame.html");

    let src_parent = match src.parent() {
        Some(p) => p,
        None => {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: src.clone(),
                message: Some("Source directory has no parent".to_string()),
            });
            return errors;
        }
    };

    let result_path_buf = src_parent
        .join(if is_dev { "dev" } else { "dist" })
        .join(result_path.trim_start_matches("/"));

    let frame_content = match fs::read_to_string(&frame_path) {
        Ok(content) => content,
        Err(e) => {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::Data,
                path: frame_path.clone(),
                message: Some(format!("Failed to read frame file: {}", e)),
            });
            return errors;
        }
    };

    let content = match fs::read_to_string(&entry_path) {
        Ok(content) => content,
        Err(e) => {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::Data,
                path: entry_path.clone(),
                message: Some(format!("Failed to read data file: {}", e)),
            });
            return errors;
        }
    };

    let processed_content = if entry_path.extension().and_then(|s| s.to_str()) == Some("md") {
        // Strip frontmatter from markdown content before rendering
        match extract_frontmatter(&content, &entry_path) {
            Ok(fm_result) => {
                errors.extend(fm_result.warnings);
                frame_content.replace(
                    "${--content}",
                    &("<markdown>\n".to_owned() + &fm_result.remaining + "</markdown>"),
                )
            }
            Err(e) => {
                // Frontmatter extraction failed — report it instead of silently using raw content
                errors.push(ProcessError {
                    error_type: e.error_type.clone(),
                    item: e.item.clone(),
                    path: entry_path.clone(),
                    message: Some(format!(
                        "Failed to extract frontmatter (using raw content as fallback): {}",
                        e.message.as_deref().unwrap_or("unknown error")
                    )),
                });
                frame_content.replace("${--content}", &content)
            }
        }
    } else {
        frame_content.replace("${--content}", &content)
    };
    let final_content = kv_replace(kv, processed_content);

    let page_result = page(src, final_content, HashSet::new());

    errors.extend(page_result.errors);

    if let Some(parent) = result_path_buf.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: parent.to_path_buf(),
                message: Some(format!("Failed to create directory structure: {}", e)),
            });
        }
    }

    // If there are errors, write an error page in dev mode or skip in build mode
    if !errors.is_empty() {
        if is_dev {
            let dev_script = make_dev_script();
            let error_html = errors_to_html(&errors, dev_script.as_deref());
            let _ = fs::write(&result_path_buf, error_html.as_bytes());
        }
        // In build mode, still write the (potentially degraded) output — errors are reported to console
    }

    let mut s = page_result.output;

    // Inject KaTeX CSS if math was rendered (unless disabled)
    if katex_assets::was_katex_used() && !katex_assets::is_katex_injection_disabled() {
        katex_assets::print_katex_message();

        if s.contains("<head>") {
            s = s.replace(
                "<head>",
                &format!("<head>\n{}", katex_assets::get_katex_css_tag()),
            );
        } else {
            s = format!("{}\n{}", katex_assets::get_katex_css_tag(), s);
        }
    }

    if is_dev && !s.contains("// * SCRIPT INCLUDED IN DEV MODE") {
        s = s.replace("<head>", &format!("<head>{}", SCRIPT));
        if let Some(ws_port) = WS_PORT.get() {
            s = s.replace("__SIMPLE_WS_PORT_PLACEHOLDER__", &ws_port.to_string());
        }
    }

    // Only write normal output if there were no errors (error page already written above)
    if errors.is_empty() {
        let output = minify(&s.into_bytes(), &minify_html::Cfg::new());

        if let Err(e) = fs::write(&result_path_buf, &output) {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: result_path_buf.clone(),
                message: Some(format!("Failed to write result file: {}", e)),
            });
        }
    }

    errors
}

fn make_dev_script() -> Option<String> {
    let ws_port = WS_PORT.get()?;
    Some(SCRIPT.replace("__SIMPLE_WS_PORT_PLACEHOLDER__", &ws_port.to_string()))
}
