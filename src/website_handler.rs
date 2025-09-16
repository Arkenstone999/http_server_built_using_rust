use super::http::{Method, Request, Response, StatusCode};
use super::server::Handler;
use std::fs;
use std::path::PathBuf;

pub struct WebsiteHandler {
    public_path: PathBuf,
}

impl WebsiteHandler {
    pub fn new(public_path: PathBuf) -> Self {
        Self { public_path }
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
            Some("pdf") => "application/pdf".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }

    fn read_file(&self, file_path: &str) -> Option<(String, String)> {
        let requested_path = self.public_path.join(file_path.trim_start_matches('/'));
        
        match fs::canonicalize(&requested_path) {
            Ok(canonical_path) => {
                if canonical_path.starts_with(&self.public_path) {
                    if canonical_path.is_file() {
                        match fs::read_to_string(&canonical_path) {
                            Ok(content) => {
                                let content_type = self.get_content_type(file_path);
                                Some((content, content_type))
                            }
                            Err(_) => None,
                        }
                    } else {
                        None
                    }
                } else {
                    eprintln!("Directory traversal attempt: {}", file_path);
                    None
                }
            }
            Err(_) => None,
        }
    }
}

impl Handler for WebsiteHandler {
    fn handle_request(&self, request: &Request) -> Response {
        match request.method() {
            Method::GET => match request.path() {
                "/" => {
                    match self.read_file("index.html") {
                        Some((content, content_type)) => Response::with_content_type(StatusCode::Ok, Some(content), content_type),
                        None => Response::new(StatusCode::NotFound, Some("Index page not found".to_string())),
                    }
                }
                "/hello" => {
                    match self.read_file("hello.html") {
                        Some((content, content_type)) => Response::with_content_type(StatusCode::Ok, Some(content), content_type),
                        None => Response::new(StatusCode::NotFound, Some("Page not found".to_string())),
                    }
                }
                path => {
                    match self.read_file(path) {
                        Some((content, content_type)) => Response::with_content_type(StatusCode::Ok, Some(content), content_type),
                        None => Response::new(StatusCode::NotFound, Some("File not found".to_string())),
                    }
                }
            },
            Method::HEAD => {
                match request.path() {
                    "/" => Response::html(StatusCode::Ok, None),
                    path => {
                        if self.read_file(path).is_some() {
                            Response::new(StatusCode::Ok, None)
                        } else {
                            Response::new(StatusCode::NotFound, None)
                        }
                    }
                }
            }
            _ => Response::new(StatusCode::NotFound, Some("Method not allowed".to_string())),
        }
    }
}