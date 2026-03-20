use comrak::markdown_to_html_with_plugins;
use comrak::plugins::syntect::SyntectAdapterBuilder;
use comrak::{options::Plugins, Options};
use fancy_regex::Regex;
use katex::{Opts, OutputType};
use once_cell::sync::Lazy;
use std::path::PathBuf;

use crate::error::{ErrorType, ProcessError, WithItem};
use crate::handlers::katex_assets;
use crate::utils::{self, ProcessResult};
use crate::IS_DEV;

static MARKDOWN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?<!<!--)<markdown>([\s\S]+?)<\/markdown>(?!-->)"#)
        .expect("Regex failed to parse. This shouldn't happen.")
});

static MATH_SPAN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<span data-math-style="(inline|display)">([\s\S]+?)</span>"#)
        .expect("Math span regex failed to parse. This shouldn't happen.")
});

static SYNTAX_HIGHLIGHTER: Lazy<comrak::plugins::syntect::SyntectAdapter> =
    Lazy::new(|| SyntectAdapterBuilder::new().css().build());

fn create_markdown_options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.math_code = true;
    options.extension.math_dollars = true;
    options.extension.superscript = true;
    options.extension.footnotes = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.highlight = true;
    options.render.r#unsafe = true;
    options
}

fn render_katex(html: &str) -> (String, Vec<ProcessError>) {
    let mut errors = Vec::new();
    let mut result = String::with_capacity(html.len() + (html.len() >> 1));
    let mut last_end = 0;
    let mut has_math = false;

    for captures in MATH_SPAN_REGEX.captures_iter(html) {
        match captures {
            Ok(cap) => {
                let mat = match cap.get(0) {
                    Some(m) => m,
                    None => continue,
                };
                let start = mat.start();
                let end = mat.end();

                result.push_str(&html[last_end..start]);

                let style = match cap.get(1) {
                    Some(m) => m.as_str(),
                    None => continue,
                };
                let latex = match cap.get(2) {
                    Some(m) => m.as_str(),
                    None => continue,
                };

                let opts = match Opts::builder()
                    .output_type(OutputType::Html)
                    .display_mode(style == "display")
                    .build()
                {
                    Ok(opts) => opts,
                    Err(e) => {
                        errors.push(ProcessError {
                            error_type: ErrorType::Other,
                            item: WithItem::None,
                            path: PathBuf::new(),
                            message: Some(format!("Failed to build KaTeX options: {:?}", e)),
                        });
                        // Keep original text
                        result.push_str(&html[start..end]);
                        last_end = end;
                        continue;
                    }
                };

                match katex::render_with_opts(latex, &opts) {
                    Ok(rendered) => {
                        result.push_str(&rendered);
                        has_math = true;
                    }
                    Err(e) => {
                        errors.push(ProcessError {
                            error_type: ErrorType::Syntax,
                            item: WithItem::None,
                            path: PathBuf::new(),
                            message: Some(format!(
                                "Failed to render LaTeX expression '{}': {}",
                                latex, e
                            )),
                        });
                        // Keep original text so it's visible something went wrong
                        result.push_str(&html[start..end]);
                    }
                }

                last_end = end;
            }
            Err(e) => {
                errors.push(ProcessError {
                    error_type: ErrorType::Other,
                    item: WithItem::None,
                    path: PathBuf::new(),
                    message: Some(format!(
                        "Regex error while scanning math expressions: {}",
                        e
                    )),
                });
            }
        }
    }

    result.push_str(&html[last_end..]);

    if has_math {
        katex_assets::mark_katex_used();
    }

    (result, errors)
}

pub fn render_markdown(input: String) -> ProcessResult {
    let mut errors = Vec::new();

    // Early return if no markdown
    if !input.contains("</markdown>") {
        return ProcessResult {
            output: input,
            errors,
        };
    }

    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&*SYNTAX_HIGHLIGHTER);
    let options = create_markdown_options();

    let is_dev = *IS_DEV.get().unwrap_or(&false);
    let mut result = String::with_capacity(input.len() + (input.len() >> 2));
    let mut last_end = 0;

    for captures in MARKDOWN_REGEX.find_iter(&input) {
        match captures {
            Ok(mat) => {
                let start = mat.start();
                let end = mat.end();

                result.push_str(&input[last_end..start]);

                let markdown_content = &input[start + 10..end - 11];
                let unindented = utils::unindent(markdown_content);
                let rendered = markdown_to_html_with_plugins(&unindented, &options, &plugins);

                // Render KaTeX math expressions
                let (rendered, katex_errors) = render_katex(&rendered);
                errors.extend(katex_errors);

                if is_dev {
                    result.push_str(r#"<div style='display: contents;' data-markdown-source=""#);
                    for ch in unindented.trim().chars() {
                        match ch {
                            '"' => result.push_str("&quot;"),
                            '&' => result.push_str("&amp;"),
                            '<' => result.push_str("&lt;"),
                            '>' => result.push_str("&gt;"),
                            _ => result.push(ch),
                        }
                    }
                    result.push_str(r#"">"#);
                    result.push_str(&rendered);
                    result.push_str("</div>");
                } else {
                    result.push_str(r#"<div style='display: contents;'>"#);
                    result.push_str(&rendered);
                    result.push_str("</div>");
                }

                last_end = end;
            }
            Err(e) => {
                errors.push(ProcessError {
                    error_type: ErrorType::Other,
                    item: WithItem::None,
                    path: PathBuf::new(),
                    message: Some(format!("Regex error while scanning markdown blocks: {}", e)),
                });
            }
        }
    }

    result.push_str(&input[last_end..]);
    ProcessResult {
        output: result,
        errors,
    }
}
