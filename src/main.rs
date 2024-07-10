mod utils;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    if args.len() < 2 {
        return Ok(());
    }

    let dir = PathBuf::from(&args[1]);

    let src = dir.join("src");
    let dist = dir.join("dist");

    let pages = src.join("pages");
    let public = src.join("public");

    if dir.join("dist").exists() {
        fs::remove_dir_all(dir.join("dist"))?;
    }
    fs::create_dir(dir.join("dist"))?;

    utils::copy_into(&public, &dist);

    utils::process_pages(&dir, &src, src.clone(), pages)?;
    Ok(())
}
