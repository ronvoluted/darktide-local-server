use ddsfile::Dds;
use image::ImageOutputFormat;
use image_dds::image_from_dds;
use mime_guess::from_ext;
use std::fs::File;
use std::io::Cursor;
use tiny_http::{Header, Request, Response, StatusCode};
use url::form_urlencoded;

pub fn handle_dds_image_request(request: &Request) -> Option<Response<Cursor<Vec<u8>>>> {
    let query_part = request.url().split('?').nth(1)?;

    let params = form_urlencoded::parse(query_part.as_bytes())
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<Vec<(String, String)>>();

    let path_param = params.iter().find(|&&(ref key, _)| key == "path")?;
    let format_param = params
        .iter()
        .find(|&&(ref key, _)| key == "format")
        .map(|(_, v)| v.as_str())
        .unwrap_or("jpg");

    let file_path = std::path::Path::new(&path_param.1);
    let mut file = File::open(file_path).ok()?;

    let dds = Dds::read(&mut file).ok()?;
    let image = image_from_dds(&dds, 0).ok()?;

    let format = match format_param.to_lowercase().as_str() {
        "png" => ImageOutputFormat::Png,
        "jpg" | "jpeg" => ImageOutputFormat::Jpeg(80),
        _ => ImageOutputFormat::Jpeg(80),
    };

    let mut buffer = Cursor::new(Vec::new());
    image.write_to(&mut buffer, format).ok()?;

    let cursor = Cursor::new(buffer.into_inner());

    let mime_type = from_ext(&format_param).first_or_octet_stream();

    let response = Response::new(
        StatusCode(200),
        vec![Header::from_bytes(&b"Content-Type"[..], mime_type.as_ref()).unwrap()],
        cursor,
        None,
        None,
    );

    Some(response)
}
