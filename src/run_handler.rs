use std::{
	io::Cursor,
	os::windows::process::CommandExt,
	process::{Command, Stdio},
};
use serde_json::json;
use tiny_http::{Request, Response, StatusCode};
use winapi::um::winbase::CREATE_NO_WINDOW;

use super::constants::{PID, SUCCESS, RunRequest};
use super::utilities::split_command;

/// Run an executable with flags and return the PID of the process
pub fn handle_run_request(request: &mut Request) -> Result<Response<Cursor<Vec<u8>>>, StatusCode> {
	let mut content = String::new();
	if request.as_reader().read_to_string(&mut content).is_err() {
			return Err(StatusCode(500));
	}

	let parsed: RunRequest = match serde_json::from_str(&content) {
			Ok(parsed) => parsed,
			Err(_) => return Err(StatusCode(400)),
	};

	let segments = split_command(&parsed.command);
	if segments.is_empty() {
			return Err(StatusCode(400));
	}

	let (executable, args) = segments.split_first().unwrap();
	match Command::new(executable)
			.args(args)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.creation_flags(CREATE_NO_WINDOW)
			.spawn()
	{
			Ok(child) => {
					let pid = child.id();
					let response_body = json!({ SUCCESS: true, PID: pid });
					let json_string = response_body.to_string();
					Ok(Response::from_string(json_string))
			}
			Err(_) => {
					let response_body = json!({ SUCCESS: false });
					let json_string = response_body.to_string();
					Ok(Response::from_string(json_string).with_status_code(StatusCode(500)))
			}
	}
}
