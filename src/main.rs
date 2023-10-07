#![windows_subsystem = "windows"]

use serde_json::from_reader;
use std::{
    env, ffi::OsStr, fs::File, io::Result as IoResult, os::windows::ffi::OsStrExt, ptr, thread,
    time::Duration,
};
use tiny_http::{Server, StatusCode};
use winapi::{
    shared::winerror::ERROR_ALREADY_EXISTS, um::errhandlingapi::GetLastError,
    um::synchapi::CreateMutexW,
};

mod constants;
mod darktide;
mod image_handler;
mod run_handler;
mod utilities;
use constants::{Config, CONFIG_NAME, DEFAULT_PORT, MUTEX_NAME};
use darktide::is_darktide_running;
use image_handler::handle_image_request;
use run_handler::handle_run_request;
use utilities::empty_response_with_status;

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

    thread::spawn(move || loop {
        if !is_darktide_running() {
            println!("Darktide.exe is not running. Shutting down.");
            std::process::exit(1);
        }
        thread::sleep(Duration::from_secs(1));
    });

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().to_string();

        if method == "GET" && url == "/shutdown" {
            let _ = request.respond(empty_response_with_status(StatusCode(200)));
            std::process::exit(0);
        }

        let response = if method == "POST" && url.starts_with("/run") {
            handle_run_request(&mut request)
                .unwrap_or_else(|status| empty_response_with_status(status))
        } else if method == "GET" && url.starts_with("/image") {
            handle_image_request(&request)
                .unwrap_or_else(|| empty_response_with_status(StatusCode(400)))
        } else {
            empty_response_with_status(StatusCode(400))
        };

        let _ = request.respond(response);
    }

    Ok(())
}
