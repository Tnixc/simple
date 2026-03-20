mod handlers {
    pub mod components;
    pub mod entries;
    pub mod frontmatter;
    pub mod katex_assets;
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
use once_cell::sync::OnceCell;
use std::{env, fs, path::PathBuf, process, time::Instant};
use utils::print_vec_errs;

pub static IS_DEV: OnceCell<bool> = OnceCell::new();

fn main() {
    let args: Vec<String> = env::args().collect();

    // Handle --version flag
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-v") {
        let version = env!("CARGO_PKG_VERSION");
        let git_hash = env!("GIT_HASH");
        println!("simple {} ({})", version, git_hash);
        return;
    }

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
            let _ = IS_DEV.set(true);
            spawn_watcher(args);
        }
        "build" => {
            let _ = IS_DEV.set(false);
            if let Err(errors) = build(args) {
                print_vec_errs(&errors);
                process::exit(1);
            }
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

fn build(args: Vec<String>) -> Result<(), Vec<ProcessError>> {
    cprintln!("<c><s>Building</></>...");
    let mut errors: Vec<ProcessError> = Vec::new();

    let start = Instant::now();

    if args.len() < 3 {
        return Ok(());
    }

    let dir = PathBuf::from(&args[2]);

    let src = dir.join("src");

    let working_dir = if *IS_DEV.get().unwrap_or(&false) {
        "dev"
    } else {
        "dist"
    };

    let dist = dir.join(working_dir);

    let pages = src.join("pages");
    let public = src.join("public");

    if !dir.join(working_dir).exists() {
        if let Err(e) = fs::create_dir(dir.join(working_dir)).map_proc_err(
            WithItem::File,
            ErrorType::Io,
            &dir.join(working_dir),
            None,
        ) {
            errors.push(e);
        }
    }

    if let Err(mut page_errors) = process_pages(&dir, &src, src.clone(), pages) {
        errors.append(&mut page_errors);
    }

    if let Err(e) = utils::copy_into(&public, &dist) {
        errors.push(e);
    }

    let duration = Instant::now().duration_since(start).as_millis();

    if errors.is_empty() {
        cprintln!("<g><s>Done</></> in {duration} ms.");
        Ok(())
    } else {
        cprintln!(
            "<y><s>Done</></> in {duration} ms with {} error{}.",
            errors.len(),
            if errors.len() == 1 { "" } else { "s" }
        );
        Err(errors)
    }
}
