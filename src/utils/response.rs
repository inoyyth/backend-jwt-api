use serde::Serialize;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub status: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(message: &str, data: T) -> Self {
        Self {
            status: true,
            message: message.to_string(),
            data: Some(data),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            status: false,
            message: message.to_string(),
            data: None,
        }
    }
}
