use color_print::cformat;
use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum ErrorType {
    Io,
    Syntax,
    Circular,
    Other,
}

#[derive(Clone, Debug)]
pub enum WithItem {
    Component,
    Template,
    Data,
    File,
    None,
}

impl fmt::Display for WithItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            WithItem::Component => "component",
            WithItem::Template => "template",
            WithItem::Data => "data",
            WithItem::File => "file or directory",
            WithItem::None => "item",
        };
        write!(f, "{}", msg)
    }
}
#[derive(Clone)]
pub struct ProcessError {
    pub error_type: ErrorType,
    pub item: WithItem,
    pub path: PathBuf,
    pub message: Option<String>,
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let item = &self.item;
        let message = &self.message;
        let msg_fmt = match message {
            Some(msg) => cformat!("<strong>{msg}</>"),
            None => String::new(),
        };
        let path = self.path.to_str().unwrap_or("<invalid-utf8-path>");
        let err_msg = match self.error_type {
            ErrorType::Io => {
                cformat!("The {item} <r>{path}</> encountered an IO error. {msg_fmt}")
            }
            ErrorType::Syntax => {
                cformat!("The {item} <r>{path}</> contains a syntax error. {msg_fmt}")
            }
            ErrorType::Circular => {
                cformat!("The {item} <r>{path}</> contains a circular dependency.")
            }
            ErrorType::Other => cformat!("Error encountered in {item} <r>{path}</>. {msg_fmt}"),
        };
        write!(f, "{err_msg}")
    }
}

impl fmt::Debug for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ProcessError")
            .field("item", &self.item)
            .field("path", &self.path)
            .field("error_type", &self.error_type)
            .field("message", &self.message)
            .finish()
    }
}

pub trait MapProcErr<T, E> {
    fn map_proc_err(
        self,
        item: WithItem,
        error_type: ErrorType,
        path: &PathBuf,
        message: Option<String>,
    ) -> Result<T, ProcessError>;
}

impl<T, E: std::fmt::Display> MapProcErr<T, E> for Result<T, E> {
    fn map_proc_err(
        self,
        item: WithItem,
        error_type: ErrorType,
        path: &PathBuf,
        message: Option<String>,
    ) -> Result<T, ProcessError> {
        self.map_err(|e| {
            let msg = message.unwrap_or_else(|| format!("{}", e));
            ProcessError {
                error_type,
                item,
                path: path.clone(),
                message: Some(msg),
            }
        })
    }
}

/// Format a ProcessError as a plain-text line (no ANSI codes) for HTML error pages.
fn error_to_plain(error: &ProcessError) -> String {
    let item = &error.item;
    let path = error.path.to_str().unwrap_or("<invalid-utf8-path>");
    let msg = error.message.as_deref().unwrap_or("");
    match error.error_type {
        ErrorType::Io => format!("[IO] The {item} '{path}' encountered an IO error. {msg}"),
        ErrorType::Syntax => {
            format!("[Syntax] The {item} '{path}' contains a syntax error. {msg}")
        }
        ErrorType::Circular => {
            format!("[Circular] The {item} '{path}' contains a circular dependency.")
        }
        ErrorType::Other => format!("[Error] Error in {item} '{path}'. {msg}"),
    }
}

/// Escape HTML special characters.
fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Generate an HTML error page for the dev server.
/// `dev_script` should be the injected websocket/reload script if in dev mode.
pub fn errors_to_html(errors: &[ProcessError], dev_script: Option<&str>) -> String {
    let mut error_items = String::new();
    for (i, err) in errors.iter().enumerate() {
        let plain = escape_html(&error_to_plain(err));
        error_items.push_str(&format!(
            "<div class=\"error\"><span class=\"index\">{}</span> {}</div>\n",
            i + 1,
            plain
        ));
    }

    let script_tag = dev_script.unwrap_or("");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Build Error</title>
{script_tag}
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    padding: 2.5rem;
    background: #111;
    color: #ccc;
    line-height: 1.6;
  }}
  h1 {{
    color: #ff6b6b;
    font-size: 1.125rem;
    font-weight: 600;
    margin-bottom: 1.5rem;
    letter-spacing: -0.01em;
  }}
  .error {{
    background: #1a1a1a;
    border-left: 3px solid #ff6b6b;
    padding: 0.75rem 1rem;
    margin-bottom: 0.5rem;
    font-size: 0.8125rem;
    white-space: pre-wrap;
    word-break: break-word;
  }}
  .index {{
    color: #ff6b6b;
    font-weight: 700;
    margin-right: 0.25rem;
  }}
</style>
</head>
<body>
<h1>Build failed — {count} error{plural}</h1>
{error_items}
</body>
</html>"##,
        count = errors.len(),
        plural = if errors.len() == 1 { "" } else { "s" },
    )
}
