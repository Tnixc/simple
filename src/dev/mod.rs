use crate::error::{ErrorType, ProcessError, WithItem};
use crate::*;
use color_print::cprintln;
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

fn dev_rebuild(res: Result<notify::Event, notify::Error>) -> Result<(), Vec<ProcessError>> {
    let args: Vec<String> = env::args().collect();
    match res {
        Ok(s) => {
            println!("");
            cprintln!("<m><s>Modified: </></>{:?}", s.paths);
            let result = build(args.clone(), true);
            return result;
        }
        Err(e) => {
            return Err(vec![ProcessError {
                error_type: ErrorType::Other,
                item: None,
                path: PathBuf::from("Watcher"),
                message: Some(format!("{e} (internal watcher error)")),
            }])
        }
    }
}

fn spawn_websocket_handler(receiver: Receiver<String>, src: PathBuf) -> () {
    let clients: Arc<Mutex<HashMap<u64, Responder>>> = Arc::new(Mutex::new(HashMap::new()));

    let event_hub = simple_websockets::launch(2727).expect("failed to listen on port 2727");

    let clients_clone = Arc::clone(&clients);
    thread::spawn(move || loop {
        let message = receiver.recv().unwrap();
        let locked_clients = clients_clone.lock().unwrap();
        match message.as_str() {
            "reload" => {
                for (_, responder) in locked_clients.iter() {
                    let json = serde_json::json!(
                        {
                            "message": "reload"
                        }
                    );
                    let signal = Message::Text(json.to_string());
                    responder.send(signal);
                }
            }
            // error case:
            _ => {
                for (_, responder) in locked_clients.iter() {
                    let json = serde_json::json!(
                        {
                            "message": message
                        }
                    );
                    let signal = Message::Text(json.to_string());
                    responder.send(signal);
                }
            }
        }
    });

    loop {
        match event_hub.poll_event() {
            Event::Connect(client_id, responder) => {
                let mut locked_clients = clients.lock().unwrap();
                locked_clients.insert(client_id, responder);
            }
            Event::Disconnect(client_id) => {
                let mut locked_clients = clients.lock().unwrap();
                locked_clients.remove(&client_id);
            }
            Event::Message(_, msg) => {
                if let Message::Text(text) = msg {
                    println!("Received websocket message: {}", text);
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if json["type"] == "markdown_update" {
                            let content = json["content"].as_str().unwrap().trim();
                            let original = json["originalContent"].as_str().unwrap().trim();

                            println!("Processing markdown update:");
                            println!("Original content: {}", original);
                            println!("New content: {}", content);

                            // Search through all files in src directory recursively using our util
                            match utils::walk_dir(&src) {
                                Ok(files) => {
                                    for path in files {
                                        if let Ok(file_content) = fs::read_to_string(&path) {
                                            println!("Checking file: {}", path.display());
                                            if file_content.contains(original) {
                                                println!("Found matching file: {}", path.display());
                                                // Replace the content
                                                let new_content =
                                                    file_content.replace(original, content);

                                                match fs::write(&path, new_content) {
                                                    Ok(_) => println!("Successfully updated file"),
                                                    Err(e) => {
                                                        eprintln!("Failed to update file: {}", e)
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(e) => eprintln!("Error walking directory: {}", e),
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn spawn_watcher(args: Vec<String>) -> () {
    cprintln!("<k!>|-----------------------------------|</>");
    cprintln!("| <s>Now serving <y><u>http://localhost:7272</></></> |");
    cprintln!("<k!>|-----------------------------------|</>");
    cprintln!("<b>The websocket port for reloading is 2727.</>");

    let dist = PathBuf::from(&args[2]).join("dev");
    let src = PathBuf::from(&args[2]).join("src");

    let (sender, receiver) = channel::<String>();

    let websocket_src = src.clone();
    thread::spawn(move || spawn_websocket_handler(receiver, websocket_src));

    let _ = build(args.clone(), true).map_err(|e| {
        utils::print_vec_errs(&e);
    });

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
                let _ = sender.send(utils::format_errs(&e));
                utils::print_vec_errs(&e);
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