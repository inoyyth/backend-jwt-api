use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use regex::Regex;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::env;

#[derive(Serialize, Deserialize, Debug)]
pub struct CloudinaryResponse {
    pub public_id: String,
    pub secure_url: String,
    pub width: u32,
    pub height: u32,
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

pub async fn upload_cloudinary(
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
