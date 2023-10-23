use crate::utilities::empty_response_with_status;
use image::GenericImageView;
use lofty::{read_from_path, Accessor, AudioFile, TaggedFileExt};
use mime_guess::mime;
use serde::Serialize;
use serde_json::to_string;
use std::{collections::HashMap, fs, io::Cursor};
use tiny_http::{Request, Response, StatusCode};
use url::form_urlencoded;

#[derive(Serialize)]
struct FileInfo {
    // General
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_modified: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,

    // Audio
    #[serde(skip_serializing_if = "Option::is_none")]
    artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channels: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    track: Option<u32>,

    // Image
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
}

impl FileInfo {
    fn is_empty(&self) -> bool {
        self.artist.is_none()
            && self.album.is_none()
            && self.channels.is_none()
            && self.duration.is_none()
            && self.sample_rate.is_none()
            && self.title.is_none()
            && self.track.is_none()
            && self.width.is_none()
            && self.height.is_none()
    }
}

pub fn handle_list_directory(request: &Request) -> Option<Response<Cursor<Vec<u8>>>> {
    let query_part = request.url().split('?').nth(1)?;

    let params: HashMap<String, String> = form_urlencoded::parse(query_part.as_bytes())
        .into_owned()
        .collect();

    let path_param = params.get("path")?;
    let general_info = params.get("general_info").map_or(false, |v| v == "true");
    let audio_info = params.get("audio_info").map_or(false, |v| v == "true");
    let image_info = params.get("image_info").map_or(false, |v| v == "true");
    let sub_directories = params.get("sub_directories").map_or(false, |v| v == "true");

    if !path_param.to_lowercase().contains("darktide") {
        return Some(empty_response_with_status(StatusCode(403)));
    }

    let path = std::path::Path::new(&path_param);
    let read_dir = fs::read_dir(path).ok()?;

    if general_info || audio_info || image_info {
        let mut files_info: Vec<FileInfo> = Vec::new();

        for entry in read_dir {
            if let Ok(entry) = entry {
                let path = entry.path();

                let mut file_info = FileInfo {
                    created_at: None,
                    file_size: None,
                    last_modified: None,
                    mime_type: None,
                    name: None,
                    r#type: None,
                    artist: None,
                    album: None,
                    channels: None,
                    duration: None,
                    sample_rate: None,
                    title: None,
                    track: None,
                    width: None,
                    height: None,
                };

                if general_info {
                    let metadata = fs::metadata(&path).ok()?;

                    file_info.created_at = Some(
                        metadata
                            .created()
                            .ok()?
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()?
                            .as_secs() as i64,
                    );

                    file_info.last_modified = Some(
                        metadata
                            .modified()
                            .ok()?
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()?
                            .as_secs() as i64,
                    );

                    if !path.is_dir() {
                        // kilobytes floored with precision of 3
                        file_info.file_size =
                            Some((metadata.len() as f64 / 1_024.0 * 1_000.0).floor() / 1_000.0);
                    };

                    file_info.mime_type = Some(
                        mime_guess::from_path(&path)
                            .first_or_octet_stream()
                            .as_ref()
                            .to_string(),
                    );

                    // Only return type if sub_directories query param is true
                    // as it would be redundant when all items are files
                    if sub_directories {
                        file_info.r#type = Some(if path.is_dir() {
                            "directory".into()
                        } else {
                            "file".into()
                        });
                    }
                }

                if path.is_file() {
                    if audio_info {
                        if let Ok(tagged_file) = read_from_path(&path) {
                            let tag = tagged_file.first_tag();

                            if let Some(tag) = tag {
                                file_info.artist = tag.artist().map(|a| a.to_string());
                                file_info.album = tag.album().map(|a| a.to_string());
                                file_info.title = tag.title().map(|a| a.to_string());
                                file_info.track = tag.track();
                            }

                            let properties = tagged_file.properties();

                            file_info.channels = properties.channels();
                            file_info.sample_rate = properties.sample_rate();

                            let duration_millis = properties.duration().as_millis();
                            let duration_secs: f64 = duration_millis as f64 / 1_000.0;
                            file_info.duration = Some((duration_secs * 1_000.0).floor() / 1_000.0);
                        }
                    }

                    if image_info {
                        let mime_guess = mime_guess::from_path(&path).first_or_octet_stream();

                        if mime_guess.type_() == mime::IMAGE {
                            if let Ok(img) = image::open(&path) {
                                let dimensions = img.dimensions();
                                file_info.width = Some(dimensions.0);
                                file_info.height = Some(dimensions.1);
                            }
                        }
                    }
                }

                if general_info || !file_info.is_empty() {
                    if sub_directories || !path.is_dir() {
                        file_info.name = path.file_name()?.to_str().map(|s| s.to_string());

                        files_info.push(file_info);
                    }
                }
            }
        }

        let json_response = to_string(&files_info).ok()?;
        let json_length = json_response.len();
        let cursor = Cursor::new(json_response.into_bytes());

        let response = Response::new(
            StatusCode(200),
            vec![tiny_http::Header::from_bytes(&b"Content-Type"[..], "application/json").unwrap()],
            cursor,
            Some(json_length),
            None,
        );

        Some(response)
    } else {
        let mut file_names = Vec::new();
        let mut dir_names = Vec::new();

        for entry in read_dir {
            if let Ok(entry) = entry {
                let path = entry.path();

                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        let mut name_string = name_str.to_string();
                        if path.is_dir() && sub_directories {
                            name_string.push('/');
                            dir_names.push(name_string);
                        } else {
                            file_names.push(name_string);
                        }
                    }
                }
            }
        }

        // Put directories at start of array
        dir_names.append(&mut file_names);

        let json_response_str = to_string(&dir_names).ok()?;
        let json_length = json_response_str.len();
        let cursor = Cursor::new(json_response_str.into_bytes());

        let response = Response::new(
            StatusCode(200),
            vec![tiny_http::Header::from_bytes(&b"Content-Type"[..], "application/json").unwrap()],
            cursor,
            Some(json_length),
            None,
        );

        Some(response)
    }
}
