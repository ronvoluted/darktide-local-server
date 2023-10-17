#![windows_subsystem = "windows"]

use crossbeam::channel::{unbounded, Receiver, Sender};
use serde::Serialize;
use serde_json::from_reader;
use std::{
    collections::HashMap, env, ffi::OsStr, fs::File, io::Result as IoResult,
    os::windows::ffi::OsStrExt, ptr, thread, time::Duration,
};
use sysinfo::Pid;
use tiny_http::{Server, StatusCode};
use url::{form_urlencoded, Url};
use winapi::{
    shared::winerror::ERROR_ALREADY_EXISTS, um::errhandlingapi::GetLastError,
    um::synchapi::CreateMutexW,
};

mod constants;
mod processes;
mod utilities;
mod handlers {
    pub mod image;
    pub mod run;
}

use constants::{Config, CONFIG_NAME, DEFAULT_PORT, MUTEX_NAME};
use processes::{is_darktide_running, is_process_running, stop_process};
use utilities::{empty_response_with_status, json_response_with_status};
use handlers::{
    image::handle_image_request,
    run::handle_run_request,
};

#[derive(Serialize)]
struct ProcessRunningResponse {
    process_is_running: bool,
}

fn main() -> IoResult<()> {
    // Named mutex for single instance check
    let mutex_name: Vec<u16> = OsStr::new(MUTEX_NAME)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        CreateMutexW(ptr::null_mut(), 0, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }
    }

    let mut bin_path = env::current_exe()?;
    bin_path.pop(); // Get directory only, not executable itself
    bin_path.push(CONFIG_NAME);

    let config: Config = match File::open(bin_path) {
        Ok(file) => from_reader(file).unwrap_or_default(),
        Err(_) => Default::default(),
    };

    let port = config.port.unwrap_or(DEFAULT_PORT);

    let server = match Server::http(format!("0.0.0.0:{}", port)) {
        Ok(server) => server,
        Err(err) => {
            eprintln!("Failed to create server: {}", err);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to create server",
            ));
        }
    };

    let (process_running_sender, process_running_receiver): (
        Sender<tiny_http::Request>,
        Receiver<tiny_http::Request>,
    ) = unbounded();

    thread::spawn(|| loop {
        if !is_darktide_running() {
            println!("Darktide.exe is not running. Shutting down.");
            std::process::exit(1);
        }
        thread::sleep(Duration::from_secs(1));
    });

    // Thread to handle /process_running requests
    thread::spawn(move || {
        for request in process_running_receiver.iter() {
            let url = request.url().to_string();
            let method = request.method().to_string();

            if method == "GET" && url.starts_with("/process_running") {
                let query_string = url.splitn(2, '?').nth(1).unwrap_or_default();
                let query: HashMap<String, String> =
                    form_urlencoded::parse(query_string.as_bytes())
                        .into_owned()
                        .collect();
                let pid: Pid = query
                    .get("pid")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.into());

                let running = is_process_running(pid);
                let response_data = ProcessRunningResponse {
                    process_is_running: running,
                };
                let response = json_response_with_status(StatusCode(200), &response_data);
                let _ = request.respond(response);
                continue;
            }
        }
    });

    // Main thread for other requests
    thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let url = request.url().to_string();
            let method = request.method().to_string();

            if method == "GET" {
                if url == "/shutdown" {
                    let _ = request.respond(empty_response_with_status(StatusCode(200)));
                    std::process::exit(0);
                }

                if url.starts_with("/process_running") {
                    // Send this request to the dedicated /process_running handling thread
                    process_running_sender.send(request).unwrap();
                    continue;
                }

                if url.starts_with("/stop_process") {
                    let full_url = format!("http://localhost{}", url);
                    if let Ok(parsed_url) = Url::parse(&full_url) {
                        if let Some(pid_str) =
                            parsed_url.query_pairs().find(|(key, _)| key == "pid")
                        {
                            let pid: u32 = pid_str.1.parse().unwrap_or(0);
                            if stop_process(pid) {
                                let _ =
                                    request.respond(empty_response_with_status(StatusCode(200)));
                                continue;
                            }
                        }
                    }
                    let _ = request.respond(empty_response_with_status(StatusCode(400)));
                    continue;
                }

                if url.starts_with("/image") {
                    let response = handle_image_request(&request)
                        .unwrap_or_else(|| empty_response_with_status(StatusCode(400)));
                    let _ = request.respond(response);
                    continue;
                }
            }

            if method == "POST" {
                if url.starts_with("/run") {
                    let response = handle_run_request(&mut request)
                        .unwrap_or_else(|status| empty_response_with_status(status));
                    let _ = request.respond(response);
                    continue;
                }
            }

            let _ = request.respond(empty_response_with_status(StatusCode(400)));
        }
    });

    loop {
        thread::sleep(Duration::from_secs(60));
    }

    Ok(())
}
