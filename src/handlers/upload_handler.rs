use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use regex::Regex;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::env;
use tokio::fs::File;
#[derive(Serialize, Deserialize, Debug)]
pub struct CloudinaryResponse {
    pub public_id: String,
    pub secure_url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: String,
}

fn generate_signature(params: &[(&str, String)], api_secret: &str) -> String {
    let mut sorted = params.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    let param_string = sorted
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let mut hasher = Sha1::new();
    hasher.update(format!("{}{}", param_string, api_secret));

    format!("{:x}", hasher.finalize())
}

pub async fn upload_base64_cloudinary(
    file: String,
) -> Result<CloudinaryResponse, Box<dyn std::error::Error>> {
    let cloud_name = env::var("CLOUDINARY_CLOUD_NAME").unwrap();
    let api_key = env::var("CLOUDINARY_API_KEY").unwrap();
    let api_secret = env::var("CLOUDINARY_API_SECRET").unwrap();

    let timestamp = Utc::now().timestamp();
    let folder = "uploads/rust".to_string();

    // signature = sha1("timestamp=xxx<api_secret>")
    let signature = generate_signature(
        &[
            ("folder", folder.clone()),
            ("timestamp", timestamp.clone().to_string()),
        ],
        &api_secret,
    );

    let form = multipart::Form::new()
        .text("file", file)
        .text("timestamp", timestamp.to_string())
        .text("api_key", api_key)
        .text("signature", signature)
        .text("folder", folder);

    let url = format!(
        "https://api.cloudinary.com/v1_1/{}/image/upload",
        cloud_name
    );

    let client = reqwest::Client::new();

    let res = client.post(url).multipart(form).send().await?;

    // handle error
    if res.status().is_client_error() || res.status().is_server_error() {
        return Err(format!("Cloudinary Error: {}", res.status()).into());
    }

    let response: CloudinaryResponse = res.json().await?;

    Ok(response)
}

fn detect_file_type(bytes: &[u8]) -> String {
    if bytes.len() < 4 {
        return "bin".to_string();
    }

    match &bytes[0..4] {
        [0xFF, 0xD8, 0xFF, 0xE0] | [0xFF, 0xD8, 0xFF, 0xE1] | [0xFF, 0xD8, 0xFF, 0xE8] => "jpg",
        [0x89, 0x50, 0x4E, 0x47] => "png",
        [0x47, 0x49, 0x46, 0x38] => "gif",
        [0x42, 0x4D, ..] => "bmp",
        [0x25, 0x50, 0x44, 0x46] => "pdf",
        [0x50, 0x4B, 0x03, 0x04] | [0x50, 0x4B, 0x05, 0x06] | [0x50, 0x4B, 0x07, 0x08] => {
            // Check if it's a DOCX or XLSX by looking at the file extension
            // or by examining the ZIP contents
            "zip" // For now, treat as zip - Cloudinary will handle it as file
        }
        [0xD0, 0xCF, 0x11, 0xE0] => "doc",
        [0x09, 0x08, 0x10, 0x00] => "xls",
        _ => {
            // Check for text files by looking for printable ASCII
            if bytes
                .iter()
                .take(100)
                .all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace())
            {
                "txt"
            } else {
                "bin"
            }
        }
    }
    .to_string()
}

pub async fn upload_cloudinary(
    mut file: File,
) -> Result<CloudinaryResponse, Box<dyn std::error::Error>> {
    let cloud_name = env::var("CLOUDINARY_CLOUD_NAME").unwrap();
    let api_key = env::var("CLOUDINARY_API_KEY").unwrap();
    let api_secret = env::var("CLOUDINARY_API_SECRET").unwrap();

    let timestamp = Utc::now().timestamp();
    let folder = "uploads/rust".to_string();

    // signature = sha1("timestamp=xxx<api_secret>")
    let signature = generate_signature(
        &[
            ("folder", folder.clone()),
            ("timestamp", timestamp.clone().to_string()),
        ],
        &api_secret,
    );

    // Read file into bytes
    let mut contents = Vec::new();
    use tokio::io::AsyncReadExt;
    file.read_to_end(&mut contents).await?;

    // Detect file type by checking magic bytes
    let file_extension = detect_file_type(&contents);
    let is_image = matches!(
        file_extension.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp"
    );

    let form = multipart::Form::new()
        .part(
            "file",
            multipart::Part::bytes(contents).file_name(format!("upload.{}", file_extension)),
        )
        .text("timestamp", timestamp.to_string())
        .text("api_key", api_key)
        .text("signature", signature)
        .text("folder", folder)
        .text("resource_type", if is_image { "image" } else { "file" });

    let url = format!("https://api.cloudinary.com/v1_1/{}/auto/upload", cloud_name);

    let client = reqwest::Client::new();

    let res = client.post(url).multipart(form).send().await?;

    // handle error
    if res.status().is_client_error() || res.status().is_server_error() {
        return Err(format!("Cloudinary Error: {}", res.status()).into());
    }

    let response: CloudinaryResponse = res.json().await?;

    Ok(response)
}

pub fn decode_image(data_url: &str) -> (String, Vec<u8>) {
    // regex to get mime type and base64 data
    let re = Regex::new(r"^data:(image/\w+);base64,(.+)$").unwrap();
    // regex to get mime type and base64 data
    let caps = re.captures(data_url).expect("Invalid data URL");
    // get mime type
    let mime = caps.get(1).unwrap().as_str();
    // get base64 data
    let base64_data = caps.get(2).unwrap().as_str();
    // decode base64 data
    let bytes = general_purpose::STANDARD.decode(base64_data).unwrap();
    // return mime type and base64 data
    (mime.to_string(), bytes)
}

// upload image base6 to folder
pub fn upload_image_to_folder(image: &str) -> String {
    let image_path = if !image.is_empty() {
        let (mime, image_data) = decode_image(image);
        let image_path = format!(
            "./uploads/{}.{}",
            Utc::now().timestamp(),
            mime.split('/').last().unwrap()
        );
        println!("Image path: {}", image_path);
        std::fs::create_dir_all("./uploads").unwrap();
        std::fs::write(&image_path, image_data).unwrap();
        image_path
    } else {
        "".to_string()
    };

    image_path.to_string()
}
