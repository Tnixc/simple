use crate::error::{Error, ErrorType, WithItem};
use crate::*;
use color_print::{cformat, cprintln};
use notify::{RecursiveMode, Watcher};
use rouille::websocket;
use rouille::Response;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use WithItem::None;

fn dev_rebuild(res: Result<notify::Event, notify::Error>) -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    match res {
        Ok(s) => {
            println!("");
            cprintln!("<m><s>Modified: </></>{:?}", s.paths);
            let result = build(args.clone(), true);
            return result;
        }
        Err(e) => {
            return Err(Error {
                error_type: ErrorType::Other,
                item: None,
                path_or_message: PathBuf::from(format!("Watch error: {e}")),
            })
        }
    }
}

fn spawn_websocket_handler(receiver: Receiver<String>) -> () {
    thread::spawn(move || {
        println!("");
        loop {
            let sig = receiver.recv().unwrap_or_default();
        }
    });
}

pub fn spawn_watcher(args: Vec<String>) -> () {
    cprintln!("<k!>|-----------------------------------|</>");
    cprintln!("| <s>Now serving <y><u>http://localhost:7272</></></> |");
    cprintln!("<k!>|-----------------------------------|</>");

    let _ = build(args.clone(), true).map_err(|e| {
        eprintln!("{}", cformat!("  <k>|</> <s><r>Build error</></>: {e}"));
    });

    let dist = PathBuf::from(&args[2]).join("dev");
    let src = PathBuf::from(&args[2]).join("src");

    // let mut watcher = notify::recommended_watcher(|res| dev_watch_handler(res)).unwrap();
    // Can't use recommended_watcher because it endlessly triggers sometimes. Probably something to do with FSEvents on macOS as that doesn't work too.

    let (sender, receiver) = channel::<String>();
    let config = notify::Config::default()
        .with_compare_contents(true)
        .with_poll_interval(Duration::from_millis(200));

    spawn_websocket_handler(receiver);

    let mut watcher = notify::PollWatcher::new(
        move |res| {
            let result = dev_rebuild(res);
            if result.is_ok() {
                let send = sender.send("reload".to_string());
                if send.is_err() {
                    let e = send.unwrap_err();
                    cprintln!("<s><y>Warning: failed to send reload signal: </></>: {e}");
                }
            } else {
                let e = result.unwrap_err();
                cprintln!("<s><r>Error</></>: {e}");
            }
        },
        config,
    )
    .unwrap();

    watcher
        .watch(&src, RecursiveMode::Recursive)
        .expect("watch failed");

    rouille::start_server("localhost:7272", move |request| {
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
        }
        Response::html("404 error").with_status_code(404)
    });
}
//
