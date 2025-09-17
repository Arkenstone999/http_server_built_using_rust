use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct SecurityConfig {
    pub rate_limit_requests: usize,
    pub rate_limit_window: Duration,
    pub allowed_file_extensions: Vec<&'static str>,
    pub allowed_hosts: Vec<&'static str>,
    pub max_path_length: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limit_requests: 100, // 100 requests per minute
            rate_limit_window: Duration::from_secs(60),
            allowed_file_extensions: vec![
                "html", "css", "js", "json", "txt", "xml",
                "png", "jpg", "jpeg", "gif", "svg", "ico", "webp"
            ],
            allowed_hosts: vec!["127.0.0.1:8080", "localhost:8080"],
            max_path_length: 255,
        }
    }
}

pub struct RateLimiter {
    requests: RwLock<HashMap<IpAddr, Vec<Instant>>>,
    config: SecurityConfig,
}

impl RateLimiter {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
            config,
        }
    }

    pub fn is_allowed(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut requests = match self.requests.write() {
            Ok(requests) => requests,
            Err(_) => return false, // Fail securely
        };
        
        // Clean up old entries periodically
        if requests.len() > 1000 {
            requests.retain(|_, times| {
                times.retain(|&time| now.duration_since(time) < self.config.rate_limit_window);
                !times.is_empty()
            });
        }
        
        let ip_requests = requests.entry(ip).or_insert_with(Vec::new);
        ip_requests.retain(|&time| now.duration_since(time) < self.config.rate_limit_window);
        
        if ip_requests.len() < self.config.rate_limit_requests {
            ip_requests.push(now);
            true
        } else {
            eprintln!("🚨 Rate limit exceeded for IP: {}", ip);
            false
        }
    }
}

pub struct SecurityValidator {
    config: SecurityConfig,
}

impl SecurityValidator {
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }

    pub fn validate_path(&self, path: &str) -> Result<(), &'static str> {
        if path.len() > self.config.max_path_length {
            return Err("Path too long");
        }

        if path.contains("..") || path.contains('\0') {
            return Err("Invalid path characters");
        }

        // Block potentially dangerous paths
        let dangerous_patterns = ["/etc/", "/proc/", "/sys/", "/dev/", "/.env", "/.git"];
        for pattern in &dangerous_patterns {
            if path.contains(pattern) {
                return Err("Forbidden path");
            }
        }

        Ok(())
    }

    pub fn validate_file_extension(&self, file_path: &str) -> bool {
        file_path.split('.').last()
            .map(|ext| self.config.allowed_file_extensions.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false)
    }

    pub fn validate_host(&self, host: Option<&str>) -> bool {
        match host {
            Some(host_header) => {
                self.config.allowed_hosts.iter().any(|&allowed| host_header == allowed)
            },
            None => true, // Allow requests without Host header for local testing
        }
    }

    pub fn sanitize_user_agent(&self, user_agent: &str) -> bool {
        // Block potentially malicious user agents
        let blocked_patterns = ["<script", "javascript:", "data:", "vbscript:", "onload="];
        !blocked_patterns.iter().any(|&pattern| user_agent.to_lowercase().contains(pattern))
    }
}