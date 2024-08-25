use color_print::cformat;
use std::fmt;
use std::path::PathBuf;

pub enum ErrorType {
    NotFound,
    Io,
    Utf8,
    Syntax,
    Circular,
    Other,
}

pub enum WithItem {
    Component,
    Template,
    Data,
    File,
    None
}

impl fmt::Display for WithItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            WithItem::Component => "component",
            WithItem::Template => "template",
            WithItem::Data => "data",
            WithItem::File => "file or directory",
            WithItem::None => "item"
        };
        write!(f, "{}", msg)
    }
}

pub struct Error {
    pub error_type: ErrorType,
    pub item: WithItem,
    pub path_or_message: PathBuf,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let item = &self.item;
        let path = &self
            .path_or_message
            .to_str()
            .to_owned()
            .expect("Couldn't turn PathBuf into string");
        let err_msg = match self.error_type {
            ErrorType::NotFound => cformat!("The {item} <r>{path}</> couldn't be found."),
            ErrorType::Io => cformat!("The {item} <r>{path}</> encountered an IO error."),
            ErrorType::Utf8 => cformat!("The {item} <r>{path}</> encountered an UTF8 error."),
            ErrorType::Syntax => cformat!("The {item} <r>{path}</> encountered a syntax error."),
            ErrorType::Circular => cformat!("The {item} <r>{path}</> contains a circular dependency."),
            ErrorType::Other => cformat!("Error: <r>{path}</>.")
        };
        write!(f, "{err_msg}")
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub trait MapPageError<T, E> {
    fn map_page_err(
        self,
        item: WithItem,
        error_type: ErrorType,
        path: &PathBuf,
    ) -> Result<T, Error>;
}

impl<T, E> MapPageError<T, E> for Result<T, E> {
    fn map_page_err(
        self,
        item: WithItem,
        error_type: ErrorType,
        path: &PathBuf,
    ) -> Result<T, Error> {
        self.map_err(|_| Error {
            error_type,
            item,
            path_or_message: path.clone(),
        })
    }
}
