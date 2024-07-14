use std::fmt;

pub enum ErrorType {
    NotFound,
    Io,
    Utf8,
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
            WithItem::Component => "Component",
            WithItem::Template => "Template",
            WithItem::Data => "Data",
            WithItem::File => "File",
        };
        write!(f, "{}", msg)
    }
}

pub struct PageHandleError {
    pub error_type: ErrorType,
    pub item: WithItem,
    pub path: String,
}

impl fmt::Display for PageHandleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let item = &self.item;
        let path = &self.path.to_owned();
        let err_msg = match self.error_type {
            ErrorType::NotFound => format!("The {item} on path {path} couldn't be found."),
            ErrorType::Io => format!("The {item} on path {path} encountered an IO error."),
            ErrorType::Utf8 => format!("The {item} on path {path} encountered an IO error."),
        };
        write!(f, "Error: {}", err_msg)
    }
}

// A unique format for dubugging output
impl fmt::Debug for PageHandleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
