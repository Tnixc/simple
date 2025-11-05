use comrak::markdown_to_html_with_plugins;
use comrak::plugins::syntect::SyntectAdapterBuilder;
use comrak::{Options, Plugins};
use fancy_regex::Regex;
use katex::{Opts, OutputType};
use once_cell::sync::Lazy;

use crate::handlers::katex_assets;
use crate::utils;
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
    options.render.unsafe_ = true;
    options
}

fn render_katex(html: &str) -> Result<String, String> {
    let mut result = String::with_capacity(html.len() + (html.len() >> 1));
    let mut last_end = 0;
    let mut has_math = false;

    for captures in MATH_SPAN_REGEX.captures_iter(html) {
        if let Ok(cap) = captures {
            let mat = match cap.get(0) {
                Some(m) => m,
                None => continue,
            };
            let start = mat.start();
            let end = mat.end();

            // Add everything before this match
            result.push_str(&html[last_end..start]);

            // Extract math style and content
            let style = match cap.get(1) {
                Some(m) => m.as_str(),
                None => continue,
            };
            let latex = match cap.get(2) {
                Some(m) => m.as_str(),
                None => continue,
            };

            // Configure KaTeX options
            let opts = Opts::builder()
                .output_type(OutputType::Html)
                .display_mode(style == "display")
                .build()
                .map_err(|e| format!("Failed to build KaTeX options: {:?}", e))?;

            // Render with KaTeX
            match katex::render_with_opts(latex, &opts) {
                Ok(rendered) => {
                    result.push_str(&rendered);
                    has_math = true;
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to render LaTeX expression '{}': {}",
                        latex, e
                    ));
                }
            }

            last_end = end;
        }
    }

    result.push_str(&html[last_end..]);

    // Mark that KaTeX was used if we rendered any math
    if has_math {
        katex_assets::mark_katex_used();
    }

    Ok(result)
}

pub fn render_markdown(input: String) -> String {
    // Early return if no markdown
    if !input.contains("</markdown>") {
        return input;
    }

    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&*SYNTAX_HIGHLIGHTER);
    let options = create_markdown_options();

    let is_dev = *IS_DEV.get().unwrap_or(&false);
    let mut result = String::with_capacity(input.len() + (input.len() >> 2));
    let mut last_end = 0;

    for captures in MARKDOWN_REGEX.find_iter(&input) {
        if let Ok(mat) = captures {
            let start = mat.start();
            let end = mat.end();

            result.push_str(&input[last_end..start]);

            let markdown_content = &input[start + 10..end - 11];
            let unindented = utils::unindent(markdown_content);
            let rendered = markdown_to_html_with_plugins(&unindented, &options, &plugins);

            // Render KaTeX math expressions
            let rendered = match render_katex(&rendered) {
                Ok(html) => html,
                Err(e) => {
                    eprintln!("KaTeX rendering error: {}", e);
                    // Return the rendered HTML without KaTeX processing on error
                    rendered
                }
            };

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
    }

    result.push_str(&input[last_end..]);
    result
}
