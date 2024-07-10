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
    let components = src.join("components");
    let pages = src.join("pages");

    if dir.join("dist").exists() {
        fs::remove_dir_all(dir.join("dist"));
    }
    fs::create_dir(dir.join("dist"));
    utils::process_pages(src, "pages");
    Ok(())
}
