mod utils;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Ok(());
    }
    let command = args[1].as_str();
    match command {
        "dev" => {
            println!("No dev yet");
            return Ok(());
        }
        "build" => build(args),
        _ => {
            println!("Unknown command");
            return Ok(());
        }
    }
}

fn build(args: Vec<String>) -> io::Result<()> {
    if args.len() < 3 {
        return Ok(());
    }
    let dir = PathBuf::from(&args[2]);

    let src = dir.join("src");
    let dist = dir.join("dist");

    let pages = src.join("pages");
    let public = src.join("public");

    if !dir.join("dist").exists() {
        fs::create_dir(dir.join("dist"))?;
    }

    for entry in dist.read_dir().unwrap() {
        if entry.as_ref().unwrap().path().is_dir() {
            fs::remove_dir_all(entry.unwrap().path())?;
        } else {
            fs::remove_file(entry.unwrap().path())?
        }
    }

    utils::copy_into(&public, &dist)?;

    utils::process_pages(&dir, &src, src.clone(), pages)?;
    Ok(())
}
