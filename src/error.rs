use std::fmt;
use color_print::cformat;
use std::path::PathBuf;

pub enum ErrorType {
    NotFound,
    Io,
    Utf8,
    Syntax,
}

pub enum WithItem {
    Component,
    Template,
    Data,
    File,
}

impl fmt::Display for WithItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            WithItem::Component => "component",
            WithItem::Template => "Template",
            WithItem::Data => "data",
            WithItem::File => "file or directory",
        };
        write!(f, "{}", msg)
    }
}

pub struct PageHandleError {
    pub error_type: ErrorType,
    pub item: WithItem,
    pub path: PathBuf,
}

impl fmt::Display for PageHandleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let item = &self.item;
        let path = &self
            .path
            .to_str()
            .to_owned()
            .expect("Couldn't turn PathBuf into string");
        let err_msg = match self.error_type {
            ErrorType::NotFound => cformat!("The {item} <r>{path}</> couldn't be found."),
            ErrorType::Io => cformat!("The {item} <r>{path}</> encountered an IO error."),
            ErrorType::Utf8 => cformat!("The {item} <r>{path}</> encountered an UTF8 error."),
            ErrorType::Syntax => cformat!("The {item} <r>{path}</> encountered a syntax error."),
        };
        write!(f, "{err_msg}")
    }
}

impl fmt::Debug for PageHandleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn rewrite_error<T, E>(
    result: Result<T, E>,
    item: WithItem,
    error_type: ErrorType,
    path: &PathBuf,
) -> Result<T, PageHandleError> {
    if result.is_err() {
        result.map_err(|_| PageHandleError {
            error_type,
            item,
            path: path.to_owned(),
        })
    } else {
        result.map_err(|_| PageHandleError {
            error_type,
            item,
            path: PathBuf::new(), // small saves
        })
    }
}
