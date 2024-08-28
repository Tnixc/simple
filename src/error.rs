use color_print::cformat;
use std::fmt;
use std::path::PathBuf;

pub enum ErrorType {
    Io,
    Syntax,
    Circular,
    Other,
}

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
            Some(msg) => format!("({msg})"),
            None => format!(""),
        };
        let path = &self
            .path
            .to_str()
            .to_owned()
            .expect("Couldn't turn PathBuf into string whilst formatting error message.");
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
        write!(f, "{:?}", self)
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
            let msg = if message.is_some() {
                message.unwrap()
            } else {
                format!("{}", e)
            };
            return ProcessError {
                error_type,
                item,
                path: path.clone(),
                message: Some(msg),
            };
        })
    }
}
