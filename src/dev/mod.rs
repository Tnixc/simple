use crate::error::{ErrorType, ProcessError, WithItem};
use crate::*;
use color_print::cprintln;
use notify::{RecursiveMode, Watcher};
use once_cell::sync::OnceCell;
use rouille::Response;
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

pub static WS_PORT: OnceCell<u16> = OnceCell::new();
pub const SCRIPT: &str = include_str!("./inline_script.html");

fn dev_rebuild(res: Result<notify::Event, notify::Error>) -> Result<(), Vec<ProcessError>> {
    let args: Vec<String> = env::args().collect();
    match res {
        Ok(s) => {
            println!();
            cprintln!("<m><s>Modified: </></>{:?}", s.paths);

            build(args.clone())
        }
        Err(e) => Err(vec![ProcessError {
            error_type: ErrorType::Other,
            item: WithItem::None,
            path: PathBuf::from("Watcher"),
            message: Some(format!("{e} (internal watcher error)")),
        }]),
    }
}

fn spawn_websocket_handler(receiver: Receiver<String>, src: PathBuf, ws_port: u16) {
    let clients: Arc<Mutex<HashMap<u64, Responder>>> = Arc::new(Mutex::new(HashMap::new()));
    let event_hub = match simple_websockets::launch(ws_port) {
        Ok(hub) => hub,
        Err(e) => {
            eprintln!("Failed to launch websocket server on port {}: {:?}", ws_port, e);
            return;
        }
    };

    // Spawn thread to handle messages from receiver
    let clients_clone = Arc::clone(&clients);
    thread::spawn(move || loop {
        let message = match receiver.recv() {
            Ok(msg) => msg,
            Err(_) => return, // Channel closed, exit thread
        };
        let locked_clients = match clients_clone.lock() {
            Ok(c) => c,
            Err(_) => return, // Mutex poisoned, exit thread
        };

        let json = serde_json::json!({
            "message": if message == "reload" { "reload" } else { &message }
        });
        let signal = Message::Text(json.to_string());

        for (_, responder) in locked_clients.iter() {
            responder.send(signal.clone());
        }
    });

    // Main event loop
    loop {
        match event_hub.poll_event() {
            Event::Connect(client_id, responder) => {
                if let Ok(mut locked_clients) = clients.lock() {
                    locked_clients.insert(client_id, responder);
                }
            }

            Event::Disconnect(client_id) => {
                if let Ok(mut locked_clients) = clients.lock() {
                    locked_clients.remove(&client_id);
                }
            }

            Event::Message(_, msg) => {
                if let Message::Text(text) = msg {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        handle_markdown_update(&json, &src);
                    }
                }
            }
        }
    }
}

fn handle_markdown_update(json: &serde_json::Value, src: &PathBuf) {
    if json["type"] == "markdown_update" {
        let content = match json["content"].as_str() {
            Some(s) => s.trim(),
            None => return,
        };
        let original = match json["originalContent"].as_str() {
            Some(s) => s.trim(),
            None => return,
        };
        if let Ok(files) = utils::walk_dir(src) {
            for path in files {
                if let Ok(file_content) = fs::read_to_string(&path) {
                    if file_content.contains(original) {
                        let new_content = file_content.replace(original, content);
                        if let Err(e) = fs::write(&path, new_content) {
                            eprintln!(
                                "{}",
                                cformat!("<s><r>Failed to update file from edit:</></>: {e}")
                            );
                        }
                        break;
                    }
                }
            }
        } else if let Err(e) = utils::walk_dir(src) {
            eprintln!("{}", cformat!("<s><r>Failed to walk directory:</></>: {e}"));
        }
    }
}

pub fn spawn_watcher(args: Vec<String>) {
    let base_preview_port = 7272;
    let base_websocket_port = 27272;

    // Find available ports
    let preview_port = utils::find_next_available_port(base_preview_port);
    let websocket_port = utils::find_next_available_port(base_websocket_port);
    let _ = WS_PORT.set(websocket_port);

    cprintln!("<k!>|------------------------------------------|</>");
    cprintln!(
        "| <s>Now serving <y><u>http://localhost:{}</></></> |",
        preview_port
    );
    cprintln!("<k!>|------------------------------------------|</>");
    cprintln!(
        "<b>The websocket port for reloading is {}.</b>",
        websocket_port
    );

    let dist = PathBuf::from(&args[2]).join("dev");
    let src = PathBuf::from(&args[2]).join("src");

    let (sender, receiver) = channel::<String>();

    let websocket_src = src.clone();
    thread::spawn(move || spawn_websocket_handler(receiver, websocket_src, websocket_port));

    let _ = build(args.clone()).map_err(|e| {
        utils::print_vec_errs(&e);
    });

    let config = notify::Config::default()
        .with_compare_contents(true)
        .with_poll_interval(Duration::from_millis(200));

    let mut watcher = match notify::PollWatcher::new(
        move |res| {
            let result = dev_rebuild(res);
            if result.is_ok() {
                if let Err(e) = sender.send("reload".to_string()) {
                    cprintln!("<s><y>Warning: failed to send reload signal: </></>: {e}");
                }
            } else if let Err(e) = result {
                let _ = sender.send(utils::format_errs(&e));
                utils::print_vec_errs(&e);
            }
        },
        config,
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to create watcher: {}", e);
            return;
        }
    };

    if let Err(e) = watcher.watch(&src, RecursiveMode::Recursive) {
        eprintln!("Failed to watch directory: {}", e);
        return;
    }

    let preview_addr = format!("0.0.0.0:{}", preview_port);

    rouille::start_server(preview_addr, move |request| {
        {
            let dist_str = match dist.to_str() {
                Some(s) => s,
                None => return Response::html("500 Internal Server Error").with_status_code(500),
            };
            let mut response = rouille::match_assets(request, dist_str);
            if request.url() == "/" {
                if let Ok(f) = fs::File::open(dist.join("index").with_extension("html")) {
                    response = Response::from_file("text/html", f);
                }
            }
            if response.is_success() {
                return response;
            }
        }
        Response::html("404 error").with_status_code(404)
    });
}
