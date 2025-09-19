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
    // Simple in-memory storage for demo
    request_count: Arc<std::sync::atomic::AtomicU64>,
}

impl WebsiteHandler {
    pub fn new(public_path: PathBuf, security_config: SecurityConfig) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(security_config.clone()));
        let security_validator = SecurityValidator::new(security_config);
        
        Self { 
            public_path,
            rate_limiter,
            security_validator,
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    // Handle API routes with simple string formatting
    fn handle_api_route(&self, request: &Request, client_ip: SocketAddr) -> Option<Response> {
        let path = request.path();
        
        // Increment request counter
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        match (request.method(), path) {
            // Simple ping endpoint
            (Method::GET, "/api/ping") => {
                let response = r#"{"status": "ok", "message": "pong"}"#;
                Some(Response::with_content_type(
                    StatusCode::Ok,
                    Some(response.to_string()),
                    "application/json; charset=utf-8".to_string(),
                ))
            },

            // Server info endpoint
            (Method::GET, "/api/info") => {
                let count = self.request_count.load(std::sync::atomic::Ordering::Relaxed);
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                let response = format!(
                    r#"{{"server": "Rust HTTP Server", "version": "{}", "requests_served": {}, "timestamp": {}}}"#,
                    env!("CARGO_PKG_VERSION"),
                    count,
                    timestamp
                );
                
                Some(Response::with_content_type(
                    StatusCode::Ok,
                    Some(response),
                    "application/json; charset=utf-8".to_string(),
                ))
            },

            // Simple user endpoint with hardcoded data
            (Method::GET, "/api/users") => {
                let users_json = r#"[
                    {"id": 1, "name": "Alice", "email": "alice@example.com"},
                    {"id": 2, "name": "Bob", "email": "bob@example.com"},
                    {"id": 3, "name": "Charlie", "email": "charlie@example.com"}
                ]"#;
                
                let response = format!(
                    r#"{{"success": true, "data": {}, "message": "Users retrieved successfully"}}"#,
                    users_json
                );
                
                Some(Response::with_content_type(
                    StatusCode::Ok,
                    Some(response),
                    "application/json; charset=utf-8".to_string(),
                ))
            },

            // Get user by ID
            (Method::GET, path) if path.starts_with("/api/users/") => {
                let user_id_str = path.trim_start_matches("/api/users/");
                
                match user_id_str.parse::<u32>() {
                    Ok(user_id) if user_id >= 1 && user_id <= 3 => {
                        let (name, email) = match user_id {
                            1 => ("Alice", "alice@example.com"),
                            2 => ("Bob", "bob@example.com"),
                            3 => ("Charlie", "charlie@example.com"),
                            _ => unreachable!(),
                        };
                        
                        let response = format!(
                            r#"{{"success": true, "data": {{"id": {}, "name": "{}", "email": "{}"}}, "message": "User found"}}"#,
                            user_id, name, email
                        );
                        
                        Some(Response::with_content_type(
                            StatusCode::Ok,
                            Some(response),
                            "application/json; charset=utf-8".to_string(),
                        ))
                    },
                    Ok(_) => {
                        let error_response = r#"{"success": false, "data": null, "message": "User not found"}"#;
                        Some(Response::with_content_type(
                            StatusCode::NotFound,
                            Some(error_response.to_string()),
                            "application/json; charset=utf-8".to_string(),
                        ))
                    },
                    Err(_) => {
                        let error_response = r#"{"success": false, "data": null, "message": "Invalid user ID"}"#;
                        Some(Response::with_content_type(
                            StatusCode::BadRequest,
                            Some(error_response.to_string()),
                            "application/json; charset=utf-8".to_string(),
                        ))
                    },
                }
            },

            // Echo endpoint for testing
            (Method::POST, "/api/echo") => {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let response = format!(
                    r#"{{"success": true, "data": {{"method": "{}", "path": "{}", "client_ip": "{}", "timestamp": {}}}, "message": "Echo successful"}}"#,
                    request.method_str(),
                    request.path(),
                    client_ip.ip(),
                    timestamp
                );
                
                Some(Response::with_content_type(
                    StatusCode::Ok,
                    Some(response),
                    "application/json; charset=utf-8".to_string(),
                ))
            },

            // Handle query parameters example
            (Method::GET, "/api/search") => {
                let query_result = match request.query_string() {
                    Some(qs) => {
                        if let Some(query) = qs.get("q") {
                            format!(r#"{{"query": "{:?}", "results": ["result1", "result2", "result3"]}}"#, query)
                        } else {
                            r#"{"error": "Missing 'q' parameter"}"#.to_string()
                        }
                    },
                    None => r#"{"error": "No query parameters provided"}"#.to_string()
                };
                
                let response = format!(
                    r#"{{"success": true, "data": {}, "message": "Search completed"}}"#,
                    query_result
                );
                
                Some(Response::with_content_type(
                    StatusCode::Ok,
                    Some(response),
                    "application/json; charset=utf-8".to_string(),
                ))
            },

            // Time endpoint
            (Method::GET, "/api/time") => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap();
                
                let response = format!(
                    r#"{{"success": true, "data": {{"unix_timestamp": {}, "milliseconds": {}}}, "message": "Current server time"}}"#,
                    now.as_secs(),
                    now.as_millis()
                );
                
                Some(Response::with_content_type(
                    StatusCode::Ok,
                    Some(response),
                    "application/json; charset=utf-8".to_string(),
                ))
            },

            // API route not found
            (_, path) if path.starts_with("/api/") => {
                let error_response = r#"{"success": false, "data": null, "message": "API endpoint not found"}"#;
                Some(Response::with_content_type(
                    StatusCode::NotFound,
                    Some(error_response.to_string()),
                    "application/json; charset=utf-8".to_string(),
                ))
            },

            // Not an API route
            _ => None,
        }
    }

    // Your existing file serving methods (unchanged)
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
                if !canonical_path.starts_with(&self.public_path) {
                    eprintln!("Directory traversal attempt blocked: {}", file_path);
                    return None;
                }

                if canonical_path.is_file() {
                    match fs::read_to_string(&canonical_path) {
                        Ok(content) => {
                            let content_type = self.get_content_type(file_path);
                            println!(" Serving file: {}", canonical_path.display());
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

        // logging
        println!(" {} {} {} from {}", 
            request.method_str(), 
            request.path(),
            if request.path().starts_with("/api/") { "" } else { "" },
            client_ip.ip()
        );

        // Try API routes first
        if let Some(api_response) = self.handle_api_route(request, client_ip) {
            return api_response;
        }

        // Fall back to static file serving for non-API routes
        match request.method() {
            Method::GET => {
                match request.path() {
                    "/" => {
                        match self.read_file("index.html") {
                            Some((content, content_type)) => {
                                Response::with_content_type(StatusCode::Ok, Some(content), content_type)
                            },
                            None => {
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
                Response::new(StatusCode::Ok, None)
            },
            _ => {
                println!(" Method {} not allowed for {}", request.method_str(), request.path());
                Response::new(StatusCode::MethodNotAllowed, Some("Method not allowed".to_string()))
            },
        }
    }
}