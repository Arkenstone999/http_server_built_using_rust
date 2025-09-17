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

    pub fn security_error(message: &str) -> Self {
        Response::new(
            StatusCode::BadRequest,
            Some(format!("Security violation: {}", message))
        )
    }

    pub fn rate_limited() -> Self {
        Response::new(
            StatusCode::TooManyRequests,
            Some("Rate limit exceeded. Please try again later.".to_string())
        )
    }

    fn get_security_headers(&self) -> String {
        // Comprehensive security headers
        format!(
            "X-Content-Type-Options: nosniff\r\n\
            X-Frame-Options: DENY\r\n\
            X-XSS-Protection: 1; mode=block\r\n\
            Referrer-Policy: strict-origin-when-cross-origin\r\n\
            Content-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; object-src 'none'; base-uri 'self'\r\n\
            Strict-Transport-Security: max-age=31536000; includeSubDomains\r\n\
            Permissions-Policy: geolocation=(), microphone=(), camera=()\r\n\
            Cache-Control: no-cache, no-store, must-revalidate\r\n\
            Pragma: no-cache\r\n\
            Expires: 0\r\n"
        )
    }

    pub async fn send(&self, stream: &mut (impl AsyncWriteExt + Unpin)) -> IoResult<()> {
        let body = match &self.body {
            Some(b) => b,
            None => "",
        };

        let security_headers = self.get_security_headers();

        let response = format!(
            "HTTP/1.1 {} {}\r\n\
            Content-Type: {}\r\n\
            Content-Length: {}\r\n\
            Connection: close\r\n\
            Server: SecureRustServer/1.0\r\n\
            {}\
            \r\n{}",
            self.status_code,
            self.status_code.reason_phrase(),
            self.content_type,
            body.len(),
            security_headers,
            body
        );

        stream.write_all(response.as_bytes()).await?;
        stream.flush().await
    }
}