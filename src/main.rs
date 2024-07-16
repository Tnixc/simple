mod error;
mod new;
mod utils;
use notify::{RecursiveMode, Watcher};
use rouille::Response;
use std::borrow::Cow;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

fn main() -> Result<(), &'static str> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        return Err("Not enough arguments. Usage: simple <operation> <dir>");
    }

    let command = args[1].as_str();
    match command {
        "dev" => {
            dev(args);
            return Ok(());
        }
        "build" => {
            build(args, false).map_err(|e| {
                println!("Error with build: {:?}", e);
                return "Build failed";
            })?;
            return Ok(());
        }
        "new" => new::new(args).map_err(|e| {
            println!("Error with scaffolding: {:?}", e);
            return "Scaffold failed";
        }),
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

    utils::process_pages(&dir, &src, src.clone(), pages, dev)
        .inspect_err(|f| eprintln!("{f}"))
        .map_err(|e| std::io::Error::new(io::ErrorKind::Other, format!("{e}")))?;

    utils::copy_into(&public, &dist)
        .inspect_err(|f| println!("Failed to copy files from `public` to `dist`: {f}"))?;

    Ok(())
}

fn dev_watch_handler(res: Result<notify::Event, notify::Error>) {
    let args: Vec<String> = env::args().collect();

    match res {
        Ok(s) => {
            // if s.kind.is_create() {
            println!("");
            println!("{:?}", s);
            build(args.clone(), true).expect("Build failed");
            // }
        }
        Err(e) => println!("watch error: {:?}", e),
    }
}

fn dev(args: Vec<String>) -> () {
    println!("|----------------------------------------|");
    println!("| Now listening on http://localhost:1717 |");
    println!("|----------------------------------------|");

    build(args.clone(), true).expect("build failed");

    let dist = PathBuf::from(&args[2]).join("dist");
    let src = PathBuf::from(&args[2]).join("src");

    // let mut watcher = notify::recommended_watcher(|res| dev_watch_handler(res)).unwrap();
    let config = notify::Config::default()
        .with_compare_contents(true)
        .with_poll_interval(Duration::from_millis(200));
    let mut watcher = notify::PollWatcher::new(|res| dev_watch_handler(res), config).unwrap();

    watcher
        .watch(&src, RecursiveMode::Recursive)
        .expect("watch failed");

    rouille::start_server("localhost:1717", move |request| {
        {
            let mut response = rouille::match_assets(request, dist.to_str().unwrap());
            if request.url() == "/" {
                let f = fs::File::open(&dist.join("index").with_extension("html"));
                if f.is_ok() {
                    response = Response::from_file("text/html", f.unwrap());
                }
            }
            if response.is_success() {
                return response;
            }
            response.headers = vec![(
                Cow::from("Cache-Control"),
                Cow::from("max-age=0, no-cache, must-revalidate, proxy-revalidate"),
            )]
        }
        Response::html("404 error").with_status_code(404)
    });
}

// <script type="text/javascript" src="https://livejs.com/live.js"></script>
