use crate::dev::{SCRIPT, WS_PORT};
use crate::error::{ErrorType, ProcessError, WithItem};
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
            path: PathBuf::from(result_path),
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

    let result_path = src_parent
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
        let content_without_frontmatter = match extract_frontmatter(&content) {
            Ok((_, remaining)) => remaining,
            Err(_) => {
                // If frontmatter extraction fails, use content as-is (might not have frontmatter)
                content.clone()
            }
        };

        frame_content.replace(
            "${--content}",
            &("<markdown>\n".to_owned() + &content_without_frontmatter + "</markdown>"),
        )
    } else {
        frame_content.replace("${--content}", &content)
    };
    let final_content = kv_replace(kv, processed_content);

    let page_result = page(src, final_content, HashSet::new());

    errors.extend(page_result.errors);

    if let Some(parent) = result_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: parent.to_path_buf(),
                message: Some(format!("Failed to create directory structure: {}", e)),
            });
        }
    }

    let mut s = page_result.output;

    // Inject KaTeX CSS if math was rendered (unless disabled)
    if katex_assets::was_katex_used() && !katex_assets::is_katex_injection_disabled() {
        // Print message once
        katex_assets::print_katex_message();

        // Inject CSS link in <head>
        if s.contains("<head>") {
            s = s.replace("<head>", &format!("<head>\n{}", katex_assets::get_katex_css_tag()));
        } else {
            // If no <head> tag, prepend to document
            s = format!("{}\n{}", katex_assets::get_katex_css_tag(), s);
        }
    }

    if is_dev && !s.contains("// * SCRIPT INCLUDED IN DEV MODE") {
        s = s.replace("<head>", &format!("<head>{}", SCRIPT));
        if let Some(ws_port) = WS_PORT.get() {
            s = s.replace("__SIMPLE_WS_PORT_PLACEHOLDER__", &ws_port.to_string());
        }
    }

    let output = minify(&s.into_bytes(), &minify_html::Cfg::spec_compliant());

    match fs::write(&result_path, &output) {
        Ok(_) => (),
        Err(e) => {
            errors.push(ProcessError {
                error_type: ErrorType::Io,
                item: WithItem::File,
                path: result_path.clone(),
                message: Some(format!("Failed to write result file: {}", e)),
            });
        }
    }

    errors
}
