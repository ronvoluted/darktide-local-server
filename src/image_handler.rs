use mime_guess::from_path;
use std::{
    fs::File,
    io::{Cursor, Read},
};
use tiny_http::{Header, Request, Response, StatusCode};
use url::form_urlencoded;

/// Return an image at the given `path` query parameter
pub fn handle_image_request(request: &Request) -> Option<Response<Cursor<Vec<u8>>>> {
    let query_part = request.url().split('?').nth(1)?;

    let params = form_urlencoded::parse(query_part.as_bytes())
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<Vec<(String, String)>>();

    let path_param = params.iter().find(|&&(ref key, _)| key == "path")?;

    let file_path = std::path::Path::new(&path_param.1);

    // Guess MIME type
    let mime_type = from_path(&file_path).first_or_octet_stream();

    let mut file = File::open(file_path).ok()?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;

    let cursor = Cursor::new(buf);

    let response = Response::new(
        StatusCode(200),
        vec![Header::from_bytes(&b"Content-Type"[..], mime_type.as_ref()).unwrap()],
        cursor,
        None,
        None,
    );

    Some(response)
}
