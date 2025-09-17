use super::http::{Method, Request, Response, StatusCode};
use super::server::Handler;
use super::security::{RateLimiter, SecurityConfig, SecurityValidator};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

pub struct WebsiteHandler {
    public_path: PathBuf,
    rate_limiter: Arc<RateLimiter>,
    security_validator: SecurityValidator,
}

impl WebsiteHandler {
    pub fn new(public_path: PathBuf, security_config: SecurityConfig) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(security_config.clone()));
        let security_validator = SecurityValidator::new(security_config);
        
        Self { 
            public_path,
            rate_limiter,
            security_validator,
        }
    }

    fn get_content_type(&self, file_path: &str) -> String {
        match file_path.split('.').last() {
            Some("html") => "text/html; charset=utf-8".to_string(),
            Some("css") => "text/css; charset=utf-8".to_string(),
            Some("js") => "application/javascript; charset=utf-8".to_string(),
            Some("json") => "application/json; charset=utf-8".to_string(),
            Some("xml") => "application/xml; charset=utf-8".to_string(),
            Some("txt") => "text/plain; charset=utf-8".to_string(),
            Some("png") => "image/png".to_string(),
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("gif") => "image/gif".to_string(),
            Some("svg") => "image/svg+xml".to_string(),
            Some("ico") => "image/x-icon".to_string(),
            Some("webp") => "image/webp".to_string(),
            Some("pdf") => "application/pdf".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }

    fn read_file(&self, file_path: &str) -> Option<(String, String)> {
        // Security validation first
        if let Err(_) = self.security_validator.validate_path(file_path) {
            return None;
        }

        if !self.security_validator.validate_file_extension(file_path) {
            eprintln!("Blocked file extension: {}", file_path);
            return None;
        }

        let requested_path = self.public_path.join(file_path.trim_start_matches('/'));
        
        match fs::canonicalize(&requested_path) {
            Ok(canonical_path) => {
                // Ensure path is within public directory
                if !canonical_path.starts_with(&self.public_path) {
                    eprintln!("Directory traversal attempt blocked: {}", file_path);
                    return None;
                }

                if canonical_path.is_file() {
                    match fs::read_to_string(&canonical_path) {
                        Ok(content) => {
                            let content_type = self.get_content_type(file_path);
                            println!("Serving file: {}", canonical_path.display());
                            Some((content, content_type))
                        }
                        Err(e) => {
                            eprintln!("Failed to read file {}: {}", canonical_path.display(), e);
                            None
                        }
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn create_safe_error_response(&self, status: StatusCode, message: &str) -> Response {
        // Don't leak internal paths or detailed error info
        let safe_message = match status {
            StatusCode::NotFound => "The requested resource was not found.".to_string(),
            StatusCode::Forbidden => "Access to this resource is forbidden.".to_string(),
            StatusCode::BadRequest => "The request was invalid.".to_string(),
            _ => message.to_string(),
        };
        
        Response::new(status, Some(safe_message))
    }
}

impl Handler for WebsiteHandler {
    fn handle_request(&self, request: &Request, client_ip: SocketAddr) -> Response {
        // Rate limiting check
        if !self.rate_limiter.is_allowed(client_ip.ip()) {
            return Response::rate_limited();
        }

        // Path security validation
        if let Err(reason) = self.security_validator.validate_path(request.path()) {
            return self.handle_security_violation(reason, client_ip);
        }

        match request.method() {
            Method::GET => {
                match request.path() {
                    "/" => {
                        match self.read_file("index.html") {
                            Some((content, content_type)) => {
                                Response::with_content_type(StatusCode::Ok, Some(content), content_type)
                            },
                            None => {
                                // Try hello.html as fallback
                                match self.read_file("hello.html") {
                                    Some((content, content_type)) => {
                                        Response::with_content_type(StatusCode::Ok, Some(content), content_type)
                                    },
                                    None => self.create_safe_error_response(StatusCode::NotFound, "Index page not found"),
                                }
                            }
                        }
                    }
                    "/hello" => {
                        match self.read_file("hello.html") {
                            Some((content, content_type)) => {
                                Response::with_content_type(StatusCode::Ok, Some(content), content_type)
                            },
                            None => self.create_safe_error_response(StatusCode::NotFound, "Page not found"),
                        }
                    }
                    path => {
                        match self.read_file(path) {
                            Some((content, content_type)) => {
                                Response::with_content_type(StatusCode::Ok, Some(content), content_type)
                            },
                            None => self.create_safe_error_response(StatusCode::NotFound, "File not found"),
                        }
                    }
                }
            },
            Method::HEAD => {
                // HEAD requests should only return headers, no body
                match request.path() {
                    "/" => {
                        if self.read_file("index.html").is_some() || self.read_file("hello.html").is_some() {
                            Response::html(StatusCode::Ok, None)
                        } else {
                            Response::new(StatusCode::NotFound, None)
                        }
                    },
                    "/hello" => {
                        if self.read_file("hello.html").is_some() {
                            Response::html(StatusCode::Ok, None)
                        } else {
                            Response::new(StatusCode::NotFound, None)
                        }
                    },
                    path => {
                        if self.read_file(path).is_some() {
                            Response::new(StatusCode::Ok, None)
                        } else {
                            Response::new(StatusCode::NotFound, None)
                        }
                    }
                }
            },
            Method::OPTIONS => {
                // Handle CORS preflight requests securely
                Response::new(StatusCode::Ok, None)
            },
            _ => {
                println!("Method {} not allowed for {}", request.method_str(), request.path());
                Response::new(StatusCode::MethodNotAllowed, Some("Method not allowed".to_string()))
            },
        }
    }
}