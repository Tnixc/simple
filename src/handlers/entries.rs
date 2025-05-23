use crate::dev::{SCRIPT, WS_PORT};
use crate::error::{ErrorType, ProcessError, WithItem};
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
    let is_dev = *IS_DEV.get().unwrap();

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
    let result_path = src
        .parent()
        .unwrap()
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
        frame_content.replace(
            "${--content}",
            &("<markdown>\n".to_owned() + &content + "</markdown>"),
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

    if is_dev && !s.contains("// * SCRIPT INCLUDED IN DEV MODE") {
        s = s.replace("<head>", &format!("<head>{}", SCRIPT));
        s = s.replace(
            "__SIMPLE_WS_PORT_PLACEHOLDER__",
            &WS_PORT.get().unwrap().to_string(),
        );
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
