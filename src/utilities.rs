use serde::Serialize;
use std::io::Cursor;
use tiny_http::{Header, Response, StatusCode};

/// Splits a string into space-separated segments, ignoring spaces in quoted substrings
///
/// # Examples
///
/// ```
/// let segments = split_command("abc \"d e f\", g h i");
/// assert_eq!(segments, vec!["abc", "\"d e f\"", "g", "h", "i"]);
/// ```
pub fn split_command(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut segment = String::new();
    let mut in_quotes = false;

    for char in command.chars() {
        match char {
            '\"' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !segment.is_empty() {
                    segments.push(segment.clone());
                    segment.clear();
                }
            }
            _ => segment.push(char),
        }
    }

    if !segment.is_empty() {
        segments.push(segment);
    }

    segments
}

/// Return a JSON response with the given data and status code
pub fn json_response_with_status<T: Serialize>(
    status: StatusCode,
    data: &T,
) -> Response<Cursor<Vec<u8>>> {
    let json_bytes = serde_json::to_vec(data).unwrap_or_else(|_| vec![]);
    let content_length = json_bytes.len();
    let cursor = Cursor::new(json_bytes);
    
    let mut response = Response::new(status, vec![], cursor, Some(content_length), None);
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    response.add_header(header);
    response
}

/// Return a response with a boolean value as a string and status code 200 OK
pub fn boolean_response_with_status(
    status: StatusCode,
    boolean: bool,
) -> Response<Cursor<Vec<u8>>> {
    let bool_str = boolean.to_string();
    let cursor = Cursor::new(bool_str.into_bytes());
    Response::new(status, vec![], cursor, None, None)
}

/// Return an empty response with the given status code
pub fn empty_response_with_status(status: StatusCode) -> Response<Cursor<Vec<u8>>> {
    Response::new(status, vec![], Cursor::new(vec![]), None, None)
}
