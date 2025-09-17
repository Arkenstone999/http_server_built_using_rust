use crate::http::{ParseError, Request, Response, StatusCode};
use std::convert::TryFrom;
use std::net::SocketAddr;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use std::sync::Arc;

pub trait Handler: Send + Sync + 'static {
    fn handle_request(&self, request: &Request, client_ip: SocketAddr) -> Response;

    fn handle_bad_request(&self, e: &ParseError) -> Response {
        eprintln!("Failed to parse request: {}", e);
        Response::new(StatusCode::BadRequest, Some("Invalid request format".to_string()))
    }

    fn handle_security_violation(&self, reason: &str, client_ip: SocketAddr) -> Response {
        eprintln!("Security violation from {}: {}", client_ip, reason);
        Response::security_error("Request blocked for security reasons")
    }
}

pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }

    pub async fn run<H: Handler>(self, handler: H) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        let handler = Arc::new(handler);
        
        println!("Listening on {}", self.addr);

        loop {
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    let handler = Arc::clone(&handler);
                    
                    tokio::spawn(async move {
                        let mut buffer = vec![0; 8192];
                        
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(10),
                            stream.read(&mut buffer)
                        ).await {
                            Ok(Ok(size)) => {
                                if size == 0 {
                                    return; // Connection closed
                                }
                                
                                buffer.truncate(size);
                                
                                let response = match Request::try_from(&buffer[..]) {
                                    Ok(request) => {
                                        println!(" {} {} {} ({})", 
                                            addr, 
                                            request.method_str(), 
                                            request.path(),
                                            size
                                        );
                                        handler.handle_request(&request, addr)
                                    },
                                    Err(e) => {
                                        eprintln!("Parse error from {}: {}", addr, e);
                                        handler.handle_bad_request(&e)
                                    },
                                };

                                if let Err(e) = response.send(&mut stream).await {
                                    eprintln!("Failed to send response to {}: {}", addr, e);
                                }
                            }
                            Ok(Err(e)) => eprintln!("Failed to read from {}: {}", addr, e),
                            Err(_) => {
                                eprintln!("Request timeout from {}", addr);
                                let timeout_response = Response::new(
                                    StatusCode::RequestTimeout, 
                                    Some("Request timeout".to_string())
                                );
                                let _ = timeout_response.send(&mut stream).await;
                            },
                        }
                    });
                }
                Err(e) => eprintln!("Failed to establish connection: {}", e),
            }
        }
    }
}