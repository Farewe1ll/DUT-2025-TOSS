use crate::http_client::HttpResponseInfo;
use crate::network::HttpRequest;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLogEntry {
	pub timestamp: chrono::DateTime<chrono::Utc>,
	pub request: HttpRequestInfo,
	pub response: Option<HttpResponseInfo>,
	pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestInfo {
	pub method: String,
	pub url: String,
	pub headers: std::collections::HashMap<String, String>,
	pub body_preview: String,
	pub source_ip: String,
	pub source_port: u16,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RequestStats {
	pub total_requests: usize,
	pub monitored_requests: usize,
	pub manual_requests: usize,
	pub replay_requests: usize,
	pub successful_requests: usize,
	pub failed_requests: usize,
	pub methods: std::collections::HashMap<String, usize>,
	pub total_response_time: u64,
	pub average_response_time: u64,
}

impl From<&HttpRequest> for HttpRequestInfo {
	fn from(req: &HttpRequest) -> Self {
		let body_preview = if req.body.len() > 1000 {
			format!("{}...", String::from_utf8_lossy(&req.body[..1000]))
		} else {
			String::from_utf8_lossy(&req.body).to_string()
		};

		Self {
			method: req.method.clone(),
			url: req.url.clone(),
			headers: req.headers.clone(),
			body_preview,
			source_ip: req.source_ip.clone(),
			source_port: req.source_port,
		}
	}
}

pub struct RequestLogger {
	log_file: Arc<Mutex<tokio::fs::File>>,
	log_file_path: String,
}

impl RequestLogger {
	pub async fn new(log_file_path: &str) -> Result<Self> {
		if let Some(parent) = std::path::Path::new(log_file_path).parent() {
			if !parent.exists() {
				tokio::fs::create_dir_all(parent).await?;
			}
		}

		let file = OpenOptions::new()
			.create(true)
			.append(true)
			.open(log_file_path)
			.await?;

		Ok(Self {
			log_file: Arc::new(Mutex::new(file)),
			log_file_path: log_file_path.to_string(),
		})
	}

	async fn log_entry<T: Serialize>(&self, entry: &T) -> Result<()> {
		let log_line = format!("{}\n", serde_json::to_string(entry)?);

		let mut file = self.log_file.lock().await;
		file.write_all(log_line.as_bytes()).await?;
		file.flush().await?;

		Ok(())
	}

	pub async fn log_request(&self, request: &HttpRequest, source: &str) -> Result<()> {
		let entry = RequestLogEntry {
			timestamp: chrono::Utc::now(),
			request: HttpRequestInfo::from(request),
			response: None,
			source: source.to_string(),
		};

		self.log_entry(&entry).await
	}

	pub async fn log_request_response(
		&self,
		request: &HttpRequest,
		response: &HttpResponseInfo,
		source: &str,
	) -> Result<()> {
		let entry = RequestLogEntry {
			timestamp: chrono::Utc::now(),
			request: HttpRequestInfo::from(request),
			response: Some(response.clone()),
			source: source.to_string(),
		};

		self.log_entry(&entry).await
	}

	pub async fn log_manual_request_response(
		&self,
		method: &str,
		url: &str,
		headers: std::collections::HashMap<String, String>,
		body: &str,
		response: &HttpResponseInfo,
	) -> Result<()> {
		let request_info = HttpRequestInfo {
			method: method.to_string(),
			url: url.to_string(),
			headers,
			body_preview: if body.len() > 1000 {
				format!("{}...", &body[..1000])
			} else {
				body.to_string()
			},
			source_ip: "manual".to_string(),
			source_port: 0,
		};

		let entry = RequestLogEntry {
			timestamp: chrono::Utc::now(),
			request: request_info,
			response: Some(response.clone()),
			source: "manual".to_string(),
		};

		self.write_log_entry(&entry).await
	}

	pub async fn log_replay_request_response(
		&self,
		request: &crate::http_client::HttpRequestBuilder,
		response: &HttpResponseInfo,
	) -> Result<()> {
		let request_info = HttpRequestInfo {
			method: request.method.clone(),
			url: request.url.clone(),
			headers: request.headers.clone(),
			body_preview: request.body.as_ref().map_or(String::new(), |b| {
				if b.len() > 1000 {
					format!("{}...", &b[..1000])
				} else {
					b.clone()
				}
			}),
			source_ip: "replay".to_string(),
			source_port: 0,
		};

		let entry = RequestLogEntry {
			timestamp: chrono::Utc::now(),
			request: request_info,
			response: Some(response.clone()),
			source: "replay".to_string(),
		};

		self.write_log_entry(&entry).await
	}

	async fn write_log_entry(&self, entry: &RequestLogEntry) -> Result<()> {
		let log_line = format!("{}\n", serde_json::to_string(entry)?);

		match self.log_file.lock().await.write_all(log_line.as_bytes()).await {
			Ok(_) => {
				if let Err(e) = self.log_file.lock().await.flush().await {
					error!("Failed to flush log file: {}", e);
				}
			}
			Err(e) => {
				error!("Failed to write to log file: {}", e);
				return Err(e.into());
			}
		}

		Ok(())
	}

	pub async fn read_recent_logs(&self, limit: usize) -> Result<Vec<RequestLogEntry>> {
		let _file_guard = self.log_file.lock().await;

		if !tokio::fs::metadata(&self.log_file_path).await.is_ok() {
			return Ok(Vec::new());
		}

		let content = match tokio::fs::read_to_string(&self.log_file_path).await {
			Ok(content) => content,
			Err(e) => {
				error!("无法读取日志文件 {}: {}", self.log_file_path, e);
				return Ok(Vec::new());
			}
		};

		if content.is_empty() {
			return Ok(Vec::new());
		}

		let lines: Vec<&str> = content.lines().collect();
		let mut entries = Vec::with_capacity(limit);

		for line in lines.iter().rev().take(limit) {
			match serde_json::from_str::<RequestLogEntry>(line) {
				Ok(entry) => entries.push(entry),
				Err(e) => {
					error!("跳过无法解析的日志条目: {}", e);
					continue;
				}
			}
		}

		entries.reverse();
		Ok(entries)
	}

	pub async fn search_logs(&self, query: &str, limit: usize) -> Result<Vec<RequestLogEntry>> {
		let _file_guard = self.log_file.lock().await;

		let content = tokio::fs::read_to_string(&self.log_file_path).await?;
		let lines: Vec<&str> = content.lines().collect();

		let mut matching_entries = Vec::new();
		let query_lower = query.to_lowercase();

		for line in lines.iter().rev() {
			if matching_entries.len() >= limit {
				break;
			}

			if let Ok(entry) = serde_json::from_str::<RequestLogEntry>(line) {

				if entry.request.url.to_lowercase().contains(&query_lower) ||
				entry.request.method.to_lowercase().contains(&query_lower) ||
				entry.request.body_preview.to_lowercase().contains(&query_lower) ||
				entry.request.headers.values().any(|v| v.to_lowercase().contains(&query_lower)) {
					matching_entries.push(entry);
				}
			}
		}

		matching_entries.reverse();
		Ok(matching_entries)
	}

	pub async fn get_request_stats(&self) -> Result<RequestStats> {
		let content = tokio::fs::read_to_string(&self.log_file_path).await?;
		let lines: Vec<&str> = content.lines().collect();

		let mut stats = RequestStats::default();

		for line in lines {
			if let Ok(entry) = serde_json::from_str::<RequestLogEntry>(line) {
				stats.total_requests += 1;

				match entry.source.as_str() {
					"monitored" => stats.monitored_requests += 1,
					"manual" => stats.manual_requests += 1,
					"replay" => stats.replay_requests += 1,
					_ => {}
				}

				*stats.methods.entry(entry.request.method).or_insert(0) += 1;

				if let Some(response) = entry.response {
					if response.status >= 200 && response.status < 300 {
						stats.successful_requests += 1;
					} else if response.status >= 400 {
						stats.failed_requests += 1;
					}

					stats.total_response_time += response.response_time_ms;
				}
			}
		}

		if stats.total_requests > 0 {
			stats.average_response_time = stats.total_response_time / (stats.total_requests as u64);
		}

		Ok(stats)
	}
}