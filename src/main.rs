mod dev;
mod error;
mod markdown;
mod new;
mod utils;
use color_print::{cformat, cprintln};
use error::MapPageError;
use error::{ErrorType, PageHandleError, WithItem};
use notify::{RecursiveMode, Watcher};
use rouille::Response;
use std::borrow::Cow;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use ErrorType::NotFound;
use WithItem::File;

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
            dev(args);
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

fn build(args: Vec<String>, dev: bool) -> Result<(), PageHandleError> {
    cprintln!("<c><s>Building</></>...");
    let s;
    if dev {
        s = "dev"
    } else {
        s = "dist"
    }
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
        fs::create_dir(dir.join(s)).map_page_err(File, NotFound, &PathBuf::from(dir.join(s)))?;
    }

    utils::process_pages(&dir, &src, src.clone(), pages, dev)?;

    utils::copy_into(&public, &dist)?;
    let duration = Instant::now().duration_since(start).as_millis();

    cprintln!("<g><s>Done</></> in {duration} ms.");
    Ok(())
}

fn dev_watch_handler(res: Result<notify::Event, notify::Error>) {
    let args: Vec<String> = env::args().collect();

    match res {
        Ok(s) => {
            println!("");
            cprintln!("<m><s>Modified: </></>{:?}", s.paths);
            let _ = build(args.clone(), true).map_err(|e| {
                eprintln!("{}", cformat!("  <k>|</> <s><r>Build error</></>: {e}"));
            });
        }
        Err(e) => eprintln!("{}", cformat!("  <k>|</> <s><r>Watch error</></>: {:?}", e)),
    }
}

fn dev(args: Vec<String>) -> () {
    cprintln!("<k!>|-----------------------------------|</>");
    cprintln!("| <s>Now serving <y><u>http://localhost:1717</></></> |");
    cprintln!("<k!>|-----------------------------------|</>");

    let _ = build(args.clone(), true).map_err(|e| {
        eprintln!("{}", cformat!("  <k>|</> <s><r>Build error</></>: {e}"));
    });

    let dist = PathBuf::from(&args[2]).join("dev");
    let src = PathBuf::from(&args[2]).join("src");

    // let mut watcher = notify::recommended_watcher(|res| dev_watch_handler(res)).unwrap();
    // Can't use recommended_watcher because it endlessly triggers sometimes. Probably something to do with FSEvents on macOS as that doesn't work too.
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
