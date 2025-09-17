#![allow(dead_code)]

use server::Server;
use std::env;
use website_handler::WebsiteHandler;
use security::SecurityConfig;

mod http;
mod server;
mod website_handler;
mod security;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let default_path = format!("{}/public", env!("CARGO_MANIFEST_DIR"));
    let public_path = env::var("PUBLIC_PATH").unwrap_or(default_path);
    
    let canonical_path = std::fs::canonicalize(&public_path)
        .map_err(|_| format!("Invalid public path: {}", public_path))?;
    
    println!("Server starting on 127.0.0.1:8080");
    println!("Serving files from: {}", canonical_path.display());
    println!("Security features enabled: Rate limiting, Security headers, File type validation");
    
    let security_config = SecurityConfig::default();
    let server = Server::new("127.0.0.1:8080".to_string());
    server.run(WebsiteHandler::new(canonical_path, security_config)).await
}