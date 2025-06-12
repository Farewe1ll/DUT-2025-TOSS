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
		let default_interface = match std::env::consts::OS {
			"macos" => "en0",
			"linux" => "eth0",
			"windows" => {
				"<请用--interface参数指定网络接口>"
			},
			_ => "en0",
		}.to_string();

		Self {
			network: NetworkConfig {
				interface: default_interface,
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

pub fn list_available_interfaces() -> Vec<String> {
	match pcap::Device::list() {
		Ok(devices) => devices.into_iter().map(|d| d.name).collect(),
		Err(_) => Vec::new(),
	}
}

pub fn interface_exists(interface: &str) -> bool {
	match pcap::Device::list() {
		Ok(devices) => devices.iter().any(|d| d.name == interface),
		Err(_) => false,
	}
}

pub fn validate_bpf_filter(filter: &str) -> bool {
	if let Ok(devices) = pcap::Device::list() {
		if let Some(device) = devices.first() {
			if let Ok(cap_result) = pcap::Capture::from_device(device.clone()) {
				if let Ok(mut cap) = cap_result.open() {
					return cap.filter(filter, true).is_ok();
				}
			}
		}
	}
	true
}