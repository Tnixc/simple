mod handlers {
    pub mod components;
    pub mod entries;
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
use error::{ErrorType, MapProcErr, ProcessError, WithItem};
use std::{env, fs, path::PathBuf, time::Instant};
use utils::print_vec_errs;

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
            let _ = build(args, false).inspect_err(|e| print_vec_errs(&e));
        }
        "new" => {
            let _ = new::new(args).inspect_err(|e| {
                eprintln!("{}", cformat!("<s><r>Scaffold error</></>: {e}"));
            });
        }
        _ => {
            println!("Unknown operation. Operations: build, dev, new");
        }
    }
}

fn build(args: Vec<String>, dev: bool) -> Result<(), Vec<ProcessError>> {
    cprintln!("<c><s>Building</></>...");
    let mut errors: Vec<ProcessError> = Vec::new();

    let start = Instant::now();
    if args.len() < 3 {
        return Ok(());
    }
    let dir = PathBuf::from(&args[2]);

    let src = dir.join("src");

    let working_dir = if dev { "dev" } else { "dist" };
    let dist = dir.join(working_dir);

    let pages = src.join("pages");
    let public = src.join("public");

    if !dir.join(working_dir).exists() {
        let _ = fs::create_dir(dir.join(working_dir))
            .map_proc_err(
                WithItem::File,
                ErrorType::Io,
                &PathBuf::from(dir.join(working_dir)),
                None,
            )
            .inspect_err(|e| errors.push((*e).clone()));
    }

    process_pages(&dir, &src, src.clone(), pages, dev)?;

    let _ = utils::copy_into(&public, &dist).inspect_err(|e| errors.push((*e).clone()));
    let duration = Instant::now().duration_since(start).as_millis();

    cprintln!("<g><s>Done</></> in {duration} ms.");
    Ok(())
}
