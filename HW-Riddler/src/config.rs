use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub proxy: ProxyConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub interface: String,
    pub monitor_filter: String,
    pub buffer_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub bind_address: IpAddr,
    pub bind_port: u16,
    pub upstream_proxy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub cookie_cache_path: String,
    pub request_log_path: String,
    pub max_cache_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                interface: "en0".to_string(),
                monitor_filter: "tcp port 80 or tcp port 443".to_string(),
                buffer_size: 65536,
            },
            proxy: ProxyConfig {
                bind_address: "127.0.0.1".parse().unwrap(),
                bind_port: 8080,
                upstream_proxy: None,
            },
            storage: StorageConfig {
                cookie_cache_path: "./cookies.json".to_string(),
                request_log_path: "./requests.log".to_string(),
                max_cache_size: 1000,
            },
        }
    }
}
