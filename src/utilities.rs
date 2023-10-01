use std::io::Cursor;
use tiny_http::{Response, StatusCode};

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

/// Return an empty response with the given status code
pub fn empty_response_with_status(status: StatusCode) -> Response<Cursor<Vec<u8>>> {
	Response::new(status, vec![], Cursor::new(vec![]), None, None)
}
