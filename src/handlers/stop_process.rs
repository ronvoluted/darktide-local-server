use tiny_http::{Request, Response, StatusCode};
use url::Url;
use crate::processes::stop_process;
use crate::utilities::empty_response_with_status;

pub fn handle_stop_process_request(request: &Request) -> Response<std::io::Cursor<Vec<u8>>> {
    let url = request.url().to_string();
    let full_url = format!("http://localhost{}", url);

    if let Ok(parsed_url) = Url::parse(&full_url) {
        if let Some((_key, value)) = parsed_url.query_pairs().find(|(key, _)| key == "pid") {
            let pid: u32 = value.parse().unwrap_or(0);
            if stop_process(pid) {
                return empty_response_with_status(StatusCode(200));
            }
        }
    }

    empty_response_with_status(StatusCode(400))
}
