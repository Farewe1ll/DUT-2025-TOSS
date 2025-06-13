use anyhow::{anyhow, Result};
use pcap::{Capture, Device};
use pnet::packet::{
	ethernet::{EtherTypes, EthernetPacket},
	ip::IpNextHeaderProtocols,
	ipv4::Ipv4Packet,
	tcp::TcpPacket,
	Packet,
};
use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn, trace};

#[derive(Debug, Clone)]
pub struct NetworkPacket {
	pub src_ip: String,
	pub dst_ip: String,
	pub src_port: u16,
	pub dst_port: u16,
	pub _protocol: String,
	pub payload: Vec<u8>,
	pub _timestamp: chrono::DateTime<chrono::Utc>,
	pub _tcp_seq: Option<u32>,
	pub _tcp_ack: Option<u32>,
	pub _tcp_flags: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
	pub method: String,
	pub url: String,
	pub headers: HashMap<String, String>,
	pub body: Vec<u8>,
	pub source_ip: String,
	pub source_port: u16,
}

pub struct PacketMonitor {
	interface: String,
	filter: String,
	packet_sender: Arc<Mutex<Option<mpsc::UnboundedSender<NetworkPacket>>>>,
	shutdown_flag: Arc<AtomicBool>,
	max_memory_usage: usize,
	retry_count: Arc<AtomicUsize>,
	is_releasing: Arc<AtomicBool>,
}

impl PacketMonitor {
	pub fn new(
		interface: String,
		filter: String,
		packet_sender: mpsc::UnboundedSender<NetworkPacket>,
	) -> Self {
		Self {
			interface,
			filter,
			packet_sender: Arc::new(Mutex::new(Some(packet_sender))),
			shutdown_flag: Arc::new(AtomicBool::new(false)),
			max_memory_usage: 100 * 1024 * 1024,
			retry_count: Arc::new(AtomicUsize::new(0)),
			is_releasing: Arc::new(AtomicBool::new(false)),
		}
	}

	pub async fn start_monitor(&self) -> Result<tokio::task::JoinHandle<()>> {
		self.retry_count.store(0, Ordering::SeqCst);

		let device = match Device::list() {
			Ok(devices) => {
				devices.into_iter()
					.find(|d| d.name == self.interface)
					.ok_or_else(|| anyhow!("Interface '{}' not found. Available interfaces: {:?}",
										self.interface,
										Device::list().map_or_else(
											|_| vec!["<error listing devices>".to_string()],
											|devs| devs.into_iter().map(|d| d.name).collect()
										)))
			},
			Err(e) => {
				let err_str = e.to_string().to_lowercase();
				if err_str.contains("permission") || err_str.contains("privileges") {
					Err(anyhow!("Insufficient privileges to list network interfaces. Please run with sudo/administrator privileges."))
				} else {
					Err(anyhow!("Failed to list network interfaces: {}", e))
				}
			}
		}?;

		info!("Starting packet monitor on interface: {} with address: {:?}",
			self.interface, device.addresses);

		let sender = {
			let guard = self.packet_sender.lock().unwrap();
			guard.as_ref().ok_or_else(|| anyhow!("Packet sender not available"))?.clone()
		};

		let retry_count = self.retry_count.clone();
		let shutdown_flag = self.shutdown_flag.clone();
		let is_releasing = self.is_releasing.clone();
		let interface = self.interface.clone();
		let filter = self.filter.clone();
		let max_memory_usage = self.max_memory_usage;

		let handle = tokio::task::spawn_blocking(move || {
			Self::run_capture_loop(
				device,
				interface,
				filter,
				shutdown_flag,
				is_releasing,
				retry_count,
				max_memory_usage,
				sender,
			)
		});

		Ok(handle)
	}

	fn run_capture_loop(
		device: Device,
		interface: String,
		filter: String,
		shutdown_flag: Arc<AtomicBool>,
		is_releasing: Arc<AtomicBool>,
		retry_count: Arc<AtomicUsize>,
		max_memory_usage: usize,
		sender: mpsc::UnboundedSender<NetworkPacket>,
	) {
		println!("网络捕获开始于接口: {}", interface);
		info!("Packet monitor loop started on interface: {}", interface);
		info!("Using filter: {}", filter);

		let mut packet_count = 0;
		let mut current_retries = 0;
		const MAX_RETRIES: usize = 3;
		let mut current_memory_usage = 0;

		let stats_interval = std::time::Duration::from_secs(5);
		let mut stats_timer = std::time::Instant::now();
		let mut packet_count_since_last_stats = 0;
		let mut http_count_since_last_stats = 0;

		let mut cap = match Self::init_capture(&device, &filter) {
			Ok(cap) => {
				println!("成功初始化网络捕获 ({})", interface);
				info!("Successfully initialized capture on {}", interface);
				cap
			},
			Err(e) => {
				error!("Failed to initialize capture on {}: {}", interface, e);
				let err_str = e.to_string().to_lowercase();
				if err_str.contains("permission") || err_str.contains("privileges") {
					error!("Error Details: Insufficient privileges. Please run with sudo/administrator privileges.");
				} else {
					error!("Error Details: {}", e);
				}
				return;
			}
		};

		let mut last_packet_time = std::time::Instant::now();

		loop {
			if last_packet_time.elapsed().as_secs() > 30 {
				info!("No packets received in the last 30 seconds. Make sure your filter '{}' is correct.", filter);
				last_packet_time = std::time::Instant::now();
			}

			if stats_timer.elapsed() >= stats_interval {
				if packet_count_since_last_stats > 0 {
					println!("已捕获 {} 个数据包 ({} 个HTTP包)",
							packet_count_since_last_stats, http_count_since_last_stats);
				}
				stats_timer = std::time::Instant::now();
				packet_count_since_last_stats = 0;
				http_count_since_last_stats = 0;
			}

			if shutdown_flag.load(Ordering::SeqCst) {
				info!("Shutdown requested, stopping packet monitor");
				break;
			}

			if is_releasing.load(Ordering::SeqCst) {
				info!("Release in progress, pausing packet processing");
				std::thread::sleep(std::time::Duration::from_millis(100));
				continue;
			}

			match cap.next_packet() {
				Ok(packet) => {
					last_packet_time = std::time::Instant::now();
					packet_count += 1;
					packet_count_since_last_stats += 1;

					if packet_count % 10 == 0 || packet_count <= 5 {
						debug!("Monitored {} packets", packet_count);
					}

					if let Some(network_packet) = Self::parse_packet(packet.data) {
						debug!("Captured packet from {}:{} to {}:{} (payload: {} bytes)",
							network_packet.src_ip, network_packet.src_port,
							network_packet.dst_ip, network_packet.dst_port,
							network_packet.payload.len());

						let is_potential_http = network_packet.dst_port == 80 ||
										network_packet.dst_port == 443 ||
										HttpParser::contains_http_method(&network_packet.payload);

						if is_potential_http {
							trace!("Potential HTTP packet detected from {}:{}",
								network_packet.src_ip, network_packet.src_port);
							http_count_since_last_stats += 1;
						}

						let packet_size =
							network_packet.payload.len() +
							network_packet.src_ip.len() +
							network_packet.dst_ip.len() +
							std::mem::size_of::<NetworkPacket>();

						if current_memory_usage + packet_size > max_memory_usage {
							warn!("Memory limit reached ({} bytes), dropping packet", max_memory_usage);
							continue;
						}

						current_memory_usage += packet_size;

						if let Err(e) = sender.send(network_packet) {
							error!("Failed to send packet: {}", e);
							break;
						} else {
							debug!("Packet sent successfully to processor");
						}
					} else {
						trace!("Received packet #{}, but does not match expected protocols", packet_count);
					}
					current_retries = 0;
				}
				Err(pcap::Error::TimeoutExpired) => {
					continue;
				}
				Err(e) => {
					error!("Error capturing packet: {}", e);
					current_retries += 1;
					retry_count.fetch_add(1, Ordering::SeqCst);

					if current_retries < MAX_RETRIES {
						warn!("Retrying capture operation ({}/{})", current_retries, MAX_RETRIES);
						std::thread::sleep(std::time::Duration::from_millis(500));

						match Self::init_capture(&device, &filter) {
							Ok(new_cap) => {
								info!("Successfully reinitialized capture");
								cap = new_cap;
							}
							Err(e) => {
								error!("Failed to reinitialize capture: {}", e);
								if current_retries >= MAX_RETRIES - 1 {
									break;
								}
							}
						}
					} else {
						error!("Maximum retries reached, stopping packet monitor");
						break;
					}
				}
			}
		}

		info!("Packet monitor loop ended, monitored {} packets total", packet_count);
		info!("Packet processing errors/retries: {}", retry_count.load(Ordering::SeqCst));
	}

	fn init_capture(device: &Device, filter: &str) -> Result<Capture<pcap::Active>> {
		let mut cap = Capture::from_device(device.clone())?
			.promisc(true)
			.snaplen(65535)
			.buffer_size(1_000_000)
			.timeout(100)
			.open()?;

		cap.filter(filter, true)?;
		Ok(cap)
	}

	pub fn shutdown(&self) {
		info!("Setting shutdown flag");
		self.shutdown_flag.store(true, Ordering::SeqCst);
	}

	pub fn release_sender(&self) {
		self.is_releasing.store(true, Ordering::SeqCst);

		std::thread::sleep(std::time::Duration::from_millis(50));

		let mut guard = self.packet_sender.lock().unwrap();
		if let Some(sender) = guard.take() {
			info!("Releasing packet sender from PacketMonitor");
			drop(sender);
		}

		self.is_releasing.store(false, Ordering::SeqCst);
	}

	fn parse_packet(data: &[u8]) -> Option<NetworkPacket> {
		let ethernet = EthernetPacket::new(data)?;

		match ethernet.get_ethertype() {
			EtherTypes::Ipv4 => {
				let ipv4 = Ipv4Packet::new(ethernet.payload())?;

				match ipv4.get_next_level_protocol() {
					IpNextHeaderProtocols::Tcp => {
						let tcp = TcpPacket::new(ipv4.payload())?;

						let tcp_seq = Some(tcp.get_sequence());
						let tcp_ack = Some(tcp.get_acknowledgement());
						let tcp_flags = Some(tcp.get_flags());

						Some(NetworkPacket {
							src_ip: ipv4.get_source().to_string(),
							dst_ip: ipv4.get_destination().to_string(),
							src_port: tcp.get_source(),
							dst_port: tcp.get_destination(),
							_protocol: "TCP".to_string(),
							payload: tcp.payload().to_vec(),
							_timestamp: chrono::Utc::now(),
							_tcp_seq: tcp_seq,
							_tcp_ack: tcp_ack,
							_tcp_flags: tcp_flags,
						})
					},
					_ => {
						debug!("Unsupported IPv4 protocol: {:?}", ipv4.get_next_level_protocol());
						None
					}
				}
			},
			EtherTypes::Ipv6 => {
				debug!("IPv6 packet detected but not yet supported");
				None
			},
			_ => {
				trace!("Unsupported EtherType: {:?}", ethernet.get_ethertype());
				None
			}
		}
	}
}

pub struct HttpParser {}

impl HttpParser {
	pub fn new() -> Self {
		Self {}
	}

	pub fn contains_http_method(data: &[u8]) -> bool {
		const HTTP_METHODS: [&[u8]; 9] = [
			b"GET ", b"POST ", b"PUT ", b"DELETE ", b"HEAD ",
			b"OPTIONS ", b"PATCH ", b"CONNECT ", b"TRACE "
		];

		if data.len() < 4 {
			return false;
		}

		for &method in &HTTP_METHODS {
			// 先检查method长度，防止后续越界
			if data.len() >= method.len() {
				// 检查是否以HTTP方法开头
				if &data[0..method.len()] == method {
					return true;
				}

				// 在小数据包中搜索HTTP方法
				if data.len() <= 64 {
					for i in 0..data.len().saturating_sub(method.len()).saturating_add(1) {
						// 确保i+method.len()不会超出数据范围
						if i + method.len() <= data.len() && &data[i..i+method.len()] == method {
							return true;
						}
					}
				}
			}
		}

		false
	}

	fn parse_http_request_from_string(data_str: &str) -> Option<HttpRequest> {
		if !data_str.contains(" HTTP/") && !data_str.contains("GET ") &&
		!data_str.contains("POST ") && !data_str.contains("PUT ") {
			return None;
		}

		let lines: Vec<&str> = data_str.lines().collect();
		if lines.is_empty() {
			return None;
		}

		let mut request_line_index = 0;
		let mut request_line_parts = Vec::new();

		const HTTP_METHODS: [&str; 9] = [
			"GET", "POST", "PUT", "DELETE", "HEAD",
			"OPTIONS", "PATCH", "CONNECT", "TRACE"
		];

		for (i, line) in lines.iter().enumerate() {
			if line.trim().is_empty() || line.len() < 5 {
				continue;
			}

			let parts: Vec<&str> = line.split_whitespace().collect();
			if parts.len() >= 3 {
				if HTTP_METHODS.contains(&parts[0]) {
					request_line_index = i;
					request_line_parts = parts;
					break;
				}
			}
		}

		if request_line_parts.len() < 3 {
			return None;
		}

		let _method = request_line_parts[0].to_string();
		let path = request_line_parts[1].to_string();

		let version = request_line_parts[2];
		if !version.starts_with("HTTP/") {
			debug!("无效的HTTP版本: {}", version);
		}

		let mut _header_end = request_line_index + 1;

		let mut headers = HashMap::new();

		for i in request_line_index + 1..lines.len() {
			let line = lines[i].trim();

			if line.is_empty() {
				_header_end = i + 1;
				break;
			}

			if line.starts_with(' ') || line.starts_with('\t') {
				if let Some(last_header) = headers.keys().last().cloned() {
					if let Some(value) = headers.get_mut(&last_header) {
						*value = format!("{} {}", value, line.trim());
					}
				}
				continue;
			}

			if let Some(colon_pos) = line.find(':') {
				if colon_pos > 0 {
					let key = line[..colon_pos].trim().to_lowercase();
					let value = line[colon_pos + 1..].trim().to_string();
					headers.insert(key, value);
				}
			}
		}

		let host = headers.get("host").cloned().unwrap_or_default();
		let scheme = if headers.get("x-forwarded-proto").map_or(false, |v| v == "https") ||
					path.starts_with("https://") {
			"https"
		} else {
			"http"
		};

		let url = if path.starts_with("http://") || path.starts_with("https://") {
			path.clone()
		} else if path.starts_with("//") {
			format!("{}:{}", scheme, path)
		} else {
			format!("{}://{}{}", scheme, host, path)
		};

		let mut _content_length = 0;
		if let Some(cl) = headers.get("content-length") {
			if let Ok(len) = cl.parse::<usize>() {
				_content_length = len;
			}
		}

		let _chunked_encoding = headers.get("transfer-encoding")
			.map_or(false, |v| v.to_lowercase().contains("chunked"));

		Some(HttpRequest {
			method: _method,
			url: url,
			headers: headers.clone(),
			body: Vec::new(),
			source_ip: String::new(),
			source_port: 0,
		})
	}

	pub fn parse_http_request(packet: &NetworkPacket) -> Option<HttpRequest> {
		debug!("Attempting to parse HTTP request from packet: {:?}", packet.src_port);

		if packet.payload.len() < 16 {
			trace!("Packet payload too small for HTTP: {} bytes", packet.payload.len());
			return None;
		}

		if !HttpParser::contains_http_method(&packet.payload) {
			trace!("No HTTP method found in payload");
			return None;
		}

		let payload_str = String::from_utf8_lossy(&packet.payload);
		debug!("Found potential HTTP data, first 50 chars: {}",
			if payload_str.len() > 50 { &payload_str[..50] } else { &payload_str });

		if let Some(mut request) = HttpParser::parse_http_request_from_string(&payload_str) {
			request.source_ip = packet.src_ip.clone();
			request.source_port = packet.src_port;
			debug!("Successfully parsed HTTP request: {} {}", request.method, request.url);
			return Some(request);
		} else {
			debug!("Failed to parse HTTP request from valid payload");
		}

		None
	}
}