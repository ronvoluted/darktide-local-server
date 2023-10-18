use serde::Serialize;
use tiny_http::{Request, Response, StatusCode};
use std::{
    collections::HashMap,
		io::Cursor,
    io::Result as IoResult,
};
use sysinfo::Pid;
use url::form_urlencoded;
use crate::utilities::json_response_with_status;

#[derive(Serialize)]
struct ProcessRunningResponse {
    process_is_running: bool,
}

pub fn handle_process_running_request(
	request: &Request,
	is_process_running_fn: impl Fn(Pid) -> bool,
) -> IoResult<Response<Cursor<Vec<u8>>>> {
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

        let running = is_process_running_fn(pid);
        let response_data = ProcessRunningResponse {
            process_is_running: running,
        };

        return Ok(json_response_with_status(StatusCode(200), &response_data));
    }
    Ok(json_response_with_status(StatusCode(400), &"Bad Request"))
}
