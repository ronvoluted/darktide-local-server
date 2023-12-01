use crate::utilities::empty_response_with_status;
use image::GenericImageView;
use lofty::{read_from_path, Accessor, AudioFile, TaggedFileExt};
use mime_guess::mime;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::to_string;
use std::{collections::HashMap, fs, io::Cursor, path::Path};
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

#[derive(Serialize)]
struct DirectoryResponse<T> {
    contents: T,
}

impl FileInfo {
    fn new() -> Self {
        FileInfo {
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
        }
    }

    fn is_empty(&self, general_info: bool) -> bool {
        if general_info {
            return self.created_at.is_none()
                && self.file_size.is_none()
                && self.last_modified.is_none()
                && self.mime_type.is_none()
                && self.name.is_none()
                && self.r#type.is_none();
        }

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

    if general_info || audio_info || image_info {
        let files_info = gather_file_info(
            &path,
            general_info,
            audio_info,
            image_info,
            sub_directories,
            "",
        );

        let response_data = DirectoryResponse {
            contents: files_info,
        };

        Some(create_json_response(response_data, StatusCode(200)))
    } else {
        let contents = list_directory_contents(path, sub_directories);
        let response_data = DirectoryResponse { contents };

        Some(create_json_response(response_data, StatusCode(200)))
    }
}

fn list_directory_contents(path: &Path, include_subdirectories: bool) -> Vec<String> {
    let mut contents = Vec::new();

    _list_directory_contents(path, include_subdirectories, &mut contents, "");

    contents
}

fn _list_directory_contents(
    path: &Path,
    include_subdirectories: bool,
    contents: &mut Vec<String>,
    prefix: &str,
) {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                let file_name = match path.file_name() {
                    Some(name) => match name.to_str() {
                        Some(name_str) => format!("{}{}", prefix, name_str),
                        None => continue, // Skip entry if not valid Unicode
                    },
                    None => continue,
                };

                if path.is_dir() && include_subdirectories {
                    let new_prefix = format!("{}/", file_name);

                    _list_directory_contents(&path, true, contents, &new_prefix);
                } else {
                    contents.push(file_name);
                }
            }
        }
    }
}

fn gather_file_info(
    path: &Path,
    general_info: bool,
    audio_info: bool,
    image_info: bool,
    sub_directories: bool,
    prefix: &str,
) -> Vec<FileInfo> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    entries
        .par_bridge()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if path.is_dir() && sub_directories {
                let new_prefix = format!("{}{}/", prefix, entry.file_name().to_string_lossy());
                Some(gather_file_info(
                    &path,
                    general_info,
                    audio_info,
                    image_info,
                    sub_directories,
                    &new_prefix,
                ))
            } else {
                let metadata = fs::metadata(&path).ok()?;
                let file_info = process_file_info(
                    &path,
                    &metadata,
                    general_info,
                    audio_info,
                    image_info,
                    &format!("{}{}", prefix, entry.file_name().to_string_lossy()),
                );

                if !file_info.is_empty(general_info) {
                    Some(vec![file_info])
                } else {
                    None
                }
            }
        })
        .flatten()
        .collect()
}

fn process_file_info(
    path: &Path,
    metadata: &fs::Metadata,
    general_info: bool,
    audio_info: bool,
    image_info: bool,
    relative_path: &str,
) -> FileInfo {
    let mut file_info = FileInfo::new();

    file_info.name = Some(relative_path.to_string());

    if general_info {
        set_general_info(&mut file_info, metadata, path);
    }

    if path.is_file() {
        if audio_info {
            set_audio_info(&mut file_info, path);
        }

        if image_info {
            set_image_info(&mut file_info, path);
        }
    }

    file_info
}

fn set_general_info(file_info: &mut FileInfo, metadata: &fs::Metadata, path: &Path) {
    file_info.created_at = metadata
        .created()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64);

    file_info.last_modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64);

    if !metadata.is_dir() {
        // File size in kilobytes (floored with precision of 3)
        file_info.file_size = Some((metadata.len() as f64 / 1024.0).round());
    }

    file_info.mime_type = Some(
        mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string(),
    );

    file_info.r#type = Some(if metadata.is_dir() {
        "directory".to_string()
    } else {
        "file".to_string()
    });
}

fn set_audio_info(file_info: &mut FileInfo, path: &Path) {
    if let Ok(tagged_file) = read_from_path(path) {
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
        file_info.duration = Some(properties.duration().as_secs_f64());
    }
}

fn set_image_info(file_info: &mut FileInfo, path: &Path) {
    let mime_type = mime_guess::from_path(path).first_or_octet_stream();

    if mime_type.type_() == mime::IMAGE {
        if let Ok(img) = image::open(path) {
            let dimensions = img.dimensions();
            file_info.width = Some(dimensions.0);
            file_info.height = Some(dimensions.1);
        }
    }
}

fn create_json_response<T: Serialize>(
    data: T,
    status_code: StatusCode,
) -> Response<Cursor<Vec<u8>>> {
    let json_response = to_string(&data).unwrap();
    let json_length = json_response.len();
    let cursor = Cursor::new(json_response.into_bytes());

    Response::new(
        status_code,
        vec![tiny_http::Header::from_bytes(&b"Content-Type"[..], "application/json").unwrap()],
        cursor,
        Some(json_length),
        None,
    )
}
