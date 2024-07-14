use custom_error::custom_error;
use std::convert::From;
use std::string::FromUtf8Error;

custom_error! {pub PageHandleError
    NotFound{type_of_thing: String, item: String} = "{type_of_thing} not found: {item}",
    IO{source: std::io::Error, item: String} = "I/O error: {source} : {item}",
    UTF8{source: FromUtf8Error} = "{source}"
}
