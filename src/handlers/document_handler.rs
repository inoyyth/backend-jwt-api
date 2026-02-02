use crate::schemas::document_schema::CompletePayload;
use axum::{extract::Multipart, response::IntoResponse};
use stringcase::snake_case;
use tokio::{fs, io::AsyncWriteExt};

pub async fn upload_chunk(mut multipart: Multipart) -> impl IntoResponse {
    let mut file_id = String::new();
    let mut chunk_index = String::new();
    let mut data = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        match field.name().unwrap() {
            "file_id" => file_id = field.text().await.unwrap(),
            "chunk_index" => chunk_index = field.text().await.unwrap(),
            "data" => data = field.bytes().await.unwrap().to_vec(),
            _ => {}
        }
    }

    let dir = format!("uploads/{}", file_id);
    fs::create_dir_all(&dir).await.unwrap();

    let path = format!("{}/chunk_{}", dir, chunk_index);
    let mut file = fs::File::create(path).await.unwrap();
    file.write_all(&data).await.unwrap();

    "Ok"
}

pub async fn complete_upload(
    axum::Json(payload): axum::Json<CompletePayload>,
) -> impl IntoResponse {
    let dir = format!("uploads/{}", payload.file_id);
    let output_path = format!("uploads/{}", snake_case(&payload.name));

    println!("dir: {}", dir);
    println!("output_path: {}", output_path);
    println!("original filename: {}", payload.name);

    let mut output = tokio::fs::File::create(&output_path).await.unwrap();

    let mut entries = Vec::new();
    let mut read_dir = match tokio::fs::read_dir(&dir).await {
        Ok(read_dir) => read_dir,
        Err(_) => {
            return "No upload directory found. Please upload chunks first.".into_response();
        }
    };

    while let Some(entry) = read_dir.next_entry().await.unwrap() {
        entries.push(entry);
    }

    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let bytes = tokio::fs::read(entry.path()).await.unwrap();
        output.write_all(&bytes).await.unwrap();
    }

    // Clean up temporary chunk directory
    tokio::fs::remove_dir_all(&dir).await.unwrap();
    println!("Cleaned up temporary directory: {}", dir);

    "merged".into_response()
}
