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
        let path = self
            .path
            .to_str()
            .unwrap_or("<invalid-utf8-path>");
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
