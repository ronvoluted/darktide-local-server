#![windows_subsystem = "windows"]

use crossbeam::channel::{unbounded, Receiver, Sender};
use lazy_static::lazy_static;
use serde::Serialize;
use serde_json::from_reader;
use std::{
    collections::HashSet, env, ffi::OsStr, fs::File, io::Result as IoResult,
    os::windows::ffi::OsStrExt, ptr, sync::Mutex, thread, time::Duration,
};
use tiny_http::{Server, StatusCode};
use winapi::{
    shared::winerror::ERROR_ALREADY_EXISTS, um::errhandlingapi::GetLastError,
    um::synchapi::CreateMutexW,
};

mod constants;
mod processes;
mod utilities;
mod handlers {
    pub mod dds_image;
    pub mod image;
    pub mod process_running;
    pub mod run;
    pub mod shutdown;
    pub mod stop_process;
}

use constants::{Config, CONFIG_NAME, DEFAULT_PORT, MUTEX_NAME};
use handlers::{
    dds_image::handle_dds_image_request, image::handle_image_request, process_running::handle_process_running_request,
    run::handle_run_request, shutdown::handle_shutdown_request,
    stop_process::handle_stop_process_request,
};
use processes::{is_darktide_running, is_process_running};
use utilities::empty_response_with_status;

#[derive(Serialize)]
struct ProcessRunningResponse {
    process_is_running: bool,
}

lazy_static! {
    static ref CREATED_PIDS: Mutex<HashSet<u32>> = Mutex::new(HashSet::new());
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
            let response = handle_process_running_request(&request, is_process_running);
            let _ = request.respond(response.unwrap_or_else(|_| empty_response_with_status(StatusCode(400))));
        }
    });

    // Main thread for other requests
    thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let url = request.url().to_string();
            let method = request.method().to_string();

            if method == "GET" {
                if url == "/shutdown" {
                    // Call the new handle_shutdown_request
                    handle_shutdown_request();
                }

                if url.starts_with("/process_running") {
                    process_running_sender.send(request).unwrap();
                    continue;
                }

                if url.starts_with("/stop_process") {
                    let response = handle_stop_process_request(&request);
                    let _ = request.respond(response);
                    continue;
                }

                if url.starts_with("/image") {
                    let response = handle_image_request(&request)
                        .unwrap_or_else(|| empty_response_with_status(StatusCode(400)));
                    let _ = request.respond(response);
                    continue;
                }

                if url.starts_with("/dds_image") {
                    let response = handle_dds_image_request(&request)
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
}
