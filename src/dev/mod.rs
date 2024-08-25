use crate::error::{ErrorType, ProcessError, WithItem};
use crate::*;
use color_print::{cformat, cprintln};
use notify::{RecursiveMode, Watcher};
use rouille::Response;
use serde_json;
use simple_websockets::{Event, Message, Responder};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use WithItem::None;

fn dev_rebuild(res: Result<notify::Event, notify::Error>) -> Result<(), ProcessError> {
    let args: Vec<String> = env::args().collect();
    match res {
        Ok(s) => {
            println!("");
            cprintln!("<m><s>Modified: </></>{:?}", s.paths);
            let result = build(args.clone(), true);
            return result;
        }
        Err(e) => {
            return Err(ProcessError {
                error_type: ErrorType::Other,
                item: None,
                path_or_message: PathBuf::from(format!("Watch error: {e}")),
            })
        }
    }
}
fn spawn_websocket_handler(receiver: Receiver<String>) -> () {
    let clients: Arc<Mutex<HashMap<u64, Responder>>> = Arc::new(Mutex::new(HashMap::new()));

    let event_hub = simple_websockets::launch(2727).expect("failed to listen on port 2727");

    let clients_clone = Arc::clone(&clients);
    thread::spawn(move || loop {
        let message = receiver.recv().unwrap();

        if message == "reload" {
            let locked_clients = clients_clone.lock().unwrap();
            for (_, responder) in locked_clients.iter() {
                let json = serde_json::json!(
                    {
                        "action": "reload"
                    }
                );
                let signal = Message::Text(json.to_string());
                responder.send(signal);
            }
        }
    });

    loop {
        match event_hub.poll_event() {
            Event::Connect(client_id, responder) => {
                println!("A client connected with id #{}", client_id);
                let mut locked_clients = clients.lock().unwrap();
                locked_clients.insert(client_id, responder);
            }
            Event::Disconnect(client_id) => {
                println!("Client #{} disconnected.", client_id);
                let mut locked_clients = clients.lock().unwrap();
                locked_clients.remove(&client_id);
            }
            _ => {}
        }
    }
}

pub fn spawn_watcher(args: Vec<String>) -> () {
    cprintln!("<k!>|-----------------------------------|</>");
    cprintln!("| <s>Now serving <y><u>http://localhost:7272</></></> |");
    cprintln!("<k!>|-----------------------------------|</>");

    let (sender, receiver) = channel::<String>();
    thread::spawn(|| spawn_websocket_handler(receiver));

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
