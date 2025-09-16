use tokio::io::{Result as IoResult, AsyncWriteExt};
use super::StatusCode;

#[derive(Debug)]
pub struct Response {
    status_code: StatusCode,
    body: Option<String>,
    content_type: String,
}

impl Response {
    pub fn new(status_code: StatusCode, body: Option<String>) -> Self {
        Response { 
            status_code, 
            body,
            content_type: "text/plain; charset=utf-8".to_string(),
        }
    }

    pub fn html(status_code: StatusCode, body: Option<String>) -> Self {
        Response {
            status_code,
            body,
            content_type: "text/html; charset=utf-8".to_string(),
        }
    }

    pub fn with_content_type(status_code: StatusCode, body: Option<String>, content_type: String) -> Self {
        Response {
            status_code,
            body,
            content_type,
        }
    }

    pub async fn send(&self, stream: &mut (impl AsyncWriteExt + Unpin)) -> IoResult<()> {
        let body = match &self.body {
            Some(b) => b,
            None => "",
        };

        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nServer: RustServer/1.0\r\n\r\n{}",
            self.status_code,
            self.status_code.reason_phrase(),
            self.content_type,
            body.len(),
            body
        );

        stream.write_all(response.as_bytes()).await?;
        stream.flush().await
    }
}