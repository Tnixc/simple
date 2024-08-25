mod handlers {
    pub mod components;
    pub mod markdown;
    pub mod pages;
    pub mod templates;
}

mod dev;
mod error;
mod new;
mod utils;

use crate::handlers::pages::process_pages;
use color_print::{cformat, cprintln};
use dev::spawn_watcher;
use error::{Error, ErrorType, MapPageError, WithItem};
use std::{env, fs, path::PathBuf, time::Instant};

fn main() -> () {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!(
            "{}",
            cformat!("<s><r>Error</></>: Not enough arguments. Usage: simple [operation] [dir]")
        );
        return;
    }

    let command = args[1].as_str();
    match command {
        "dev" => {
            spawn_watcher(args);
        }
        "build" => {
            let _ = build(args, false).map_err(|e| {
                eprintln!("{}", cformat!("<s><r>Build error</></>: {e}"));
            });
        }
        "new" => {
            let _ = new::new(args).map_err(|e| {
                eprintln!("{}", cformat!("<s><r>Scaffold error</></>: {e}"));
            });
        }
        _ => {
            println!("Unknown operation. Operations: build, dev, new");
        }
    }
}

fn build(args: Vec<String>, dev: bool) -> Result<(), Error> {
    cprintln!("<c><s>Building</></>...");

    let s = if dev { "dev" } else { "dist" };

    let start = Instant::now();
    if args.len() < 3 {
        return Ok(());
    }
    let dir = PathBuf::from(&args[2]);

    let src = dir.join("src");
    let dist = dir.join(s);

    let pages = src.join("pages");
    let public = src.join("public");

    if !dir.join(s).exists() {
        fs::create_dir(dir.join(s)).map_page_err(
            WithItem::File,
            ErrorType::NotFound,
            &PathBuf::from(dir.join(s)),
        )?;
    }

    process_pages(&dir, &src, src.clone(), pages, dev)?;

    utils::copy_into(&public, &dist)?;
    let duration = Instant::now().duration_since(start).as_millis();

    cprintln!("<g><s>Done</></> in {duration} ms.");
    Ok(())
}
