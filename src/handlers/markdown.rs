use comrak::markdown_to_html_with_plugins;
use comrak::plugins::syntect::SyntectAdapterBuilder;
use comrak::Options;
use comrak::Plugins;
use fancy_regex::Regex;
use lazy_static::lazy_static;

use crate::utils;

const MARKDOWN_ELEMENT_PATTERN: &str = r#"(?<!<!--)<markdown>([\s\S]+?)<\/markdown>(?!-->)"#;

lazy_static! {
    static ref MARKDOWN_REGEX: Regex = Regex::new(MARKDOWN_ELEMENT_PATTERN)
        .expect("Regex failed to parse. This shouldn't happen.");
}

pub fn render_markdown(mut string: String) -> String {
    let codefence_syntax_highlighter = SyntectAdapterBuilder::new().css();
    let mut plugins = Plugins::default();
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

    let built = &codefence_syntax_highlighter.build();
    plugins.render.codefence_syntax_highlighter = Some(built);

    for f in MARKDOWN_REGEX.find_iter(&string.to_owned()) {
        if f.is_ok() {
            let found = f.unwrap().as_str();
            let res = found
                .trim_start_matches("<markdown>")
                .trim_end_matches("</markdown>");
            let content = utils::unindent(res);
            // Store the original markdown in a data attribute
            let rendered = format!(
                r#"<div style='display: contents;' data-markdown-source="{}">{}</div>"#,
                content.replace("\"", "&quot;").trim(),
                markdown_to_html_with_plugins(&content, &options, &plugins)
            );
            string = string.replacen(found, &rendered, 1);
        }
    }
    return string;
}