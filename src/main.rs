mod error;
mod utils;
use notify::{RecursiveMode, Watcher};
use rouille::Response;
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
            dev(args);
            return Ok(());
        }
        "build" => build(args, false),
        _ => {
            println!("Unknown command");
            return Ok(());
        }
    }
}

fn build(args: Vec<String>, dev: bool) -> io::Result<()> {
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

    let _ =
        utils::process_pages(&dir, &src, src.clone(), pages, dev).inspect_err(|f| eprintln!("{f}"));
    Ok(())
}

fn dev_watch_handler(res: Result<notify::Event, notify::Error>) {
    let args: Vec<String> = env::args().collect();

    match res {
        Ok(_) => {
            build(args.clone(), true).expect("Build failed");
        }
        Err(e) => println!("watch error: {:?}", e),
    }
}

fn dev(args: Vec<String>) -> () {
    println!("|---------------------------------|");
    println!("| Now listening on localhost:1717 |");
    println!("|---------------------------------|");

    build(args.clone(), true).expect("build failed");

    let dist = PathBuf::from(&args[2]).join("dist");
    let src = PathBuf::from(&args[2]).join("src");

    let mut watcher = notify::recommended_watcher(|res| dev_watch_handler(res)).unwrap();

    watcher
        .watch(&src, RecursiveMode::Recursive)
        .expect("watch failed");

    rouille::start_server("localhost:1717", move |request| {
        {
            let mut response = rouille::match_assets(request, dist.to_str().unwrap());
            if request.url() == "/" {
                let f = fs::File::open(&dist.join("index").with_extension("html"));
                response = Response::from_file("text/html", f.unwrap());
            }
            if response.is_success() {
                return response;
            }
        }
        Response::html("404 error").with_status_code(404)
    });
}

// <script type="text/javascript" src="https://livejs.com/live.js"></script>
