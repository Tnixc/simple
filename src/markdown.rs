use comrak::markdown_to_html_with_plugins;
use comrak::plugins::syntect::SyntectAdapterBuilder;
use comrak::Options;
use comrak::Plugins;
use fancy_regex::Regex;

const MARKDOWN_ELEMENT_PATTERN: &str = r#"(?<!<!--)<markdown>([\s\S]+?)<\/markdown>(?!-->)"#;

pub fn markdown_element(mut string: String) -> String {
    let codefence_syntax_highlighter = SyntectAdapterBuilder::new().theme("base16-ocean.dark");
    let mut plugins = Plugins::default();
    let built = &codefence_syntax_highlighter.build();
    plugins.render.codefence_syntax_highlighter = Some(built);

    let re_markdown = Regex::new(MARKDOWN_ELEMENT_PATTERN)
        .expect("Regex failed to parse. This shouldn't happen.");

    for f in re_markdown.find_iter(&string.to_owned()) {
        if f.is_ok() {
            let found = f.unwrap().as_str();
            let res = found
                .trim_start_matches("<markdown>")
                .trim_end_matches("</markdown>")
                .lines()
                .map(|f| f.trim_start().to_owned() + "\n")
                .collect::<String>();
            let rendered = &markdown_to_html_with_plugins(&res, &Options::default(), &plugins);
            string = string.replace(found, rendered);
        }
    }
    return string;
}
