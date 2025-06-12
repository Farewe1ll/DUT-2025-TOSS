use crate::cookie_manager::CookieManager;
use anyhow::Result;
use reqwest::{header::HeaderMap, Client, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestBuilder {
	pub method: String,
	pub url: String,
	pub headers: HashMap<String, String>,
	pub body: Option<String>,
	pub timeout_seconds: u64,
	pub follow_redirects: bool,
	pub verify_ssl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponseInfo {
	pub status: u16,
	pub headers: HashMap<String, String>,
	pub body: String,
	pub cookies: Vec<String>,
	pub response_time_ms: u64,
	pub final_url: String,
}

pub struct HttpClient {
	client: Client,
	cookie_manager: Arc<CookieManager>,
}

impl HttpClient {
	pub fn new(cookie_manager: Arc<CookieManager>) -> Result<Self> {
		let client = Client::builder()
			.timeout(Duration::from_secs(30))
			.connect_timeout(Duration::from_secs(10))
			.danger_accept_invalid_certs(false)
			.redirect(reqwest::redirect::Policy::limited(10))
			.user_agent("HW-Riddler/1.0")
			.build()?;

		Ok(Self {
			client,
			cookie_manager,
		})
	}

	pub async fn send_request(&self, request: HttpRequestBuilder) -> Result<HttpResponseInfo> {
		let start_time = std::time::Instant::now();

		let url = Url::parse(&request.url)?;

		let method = match request.method.to_uppercase().as_str() {
			"GET" => Method::GET,
			"POST" => Method::POST,
			"PUT" => Method::PUT,
			"DELETE" => Method::DELETE,
			"HEAD" => Method::HEAD,
			"OPTIONS" => Method::OPTIONS,
			"PATCH" => Method::PATCH,
			_ => Method::GET,
		};

		let mut headers = HeaderMap::new();
		for (key, value) in &request.headers {
			if let (Ok(header_name), Ok(header_value)) = (
				reqwest::header::HeaderName::from_bytes(key.as_bytes()),
				reqwest::header::HeaderValue::from_str(value),
			) {
				headers.insert(header_name, header_value);
			}
		}

		let cookies = self.cookie_manager.get_cookies_for_url(&url);
		if !cookies.is_empty() {
			let cookie_header = cookies.join("; ");
			if let Ok(cookie_value) = reqwest::header::HeaderValue::from_str(&cookie_header) {
				headers.insert(reqwest::header::COOKIE, cookie_value);
			}
		}

		let mut req_builder = self
			.client
			.request(method, url.clone())
			.headers(headers)
			.timeout(Duration::from_secs(request.timeout_seconds));

		if let Some(body) = &request.body {
			req_builder = req_builder.body(body.clone());
		}
		info!("Sending {} request to {}", request.method, request.url);

		let response = tokio::time::timeout(
			Duration::from_secs(request.timeout_seconds.max(5)),
			req_builder.send()
		).await
		.map_err(|_| anyhow::anyhow!("Request timed out after {} seconds", request.timeout_seconds))??;
		let final_url = response.url().to_string();
		let status = response.status().as_u16();

		let mut response_headers = HashMap::new();
		for (key, value) in response.headers() {
			if let Ok(value_str) = value.to_str() {
				response_headers.insert(key.to_string(), value_str.to_string());
			}
		}

		let mut response_cookies = Vec::new();
		for cookie_header in response.headers().get_all(reqwest::header::SET_COOKIE) {
			if let Ok(cookie_str) = cookie_header.to_str() {
				response_cookies.push(cookie_str.to_string());

				if let Err(e) = self.cookie_manager.add_cookie(&url, cookie_str) {
					error!("Failed to store cookie: {}", e);
				}
			}
		}

		let body = tokio::time::timeout(
			Duration::from_secs(30),
			response.text()
		).await
		.map_err(|_| anyhow::anyhow!("Timed out reading response body"))?
		.map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

		let response_time = start_time.elapsed().as_millis() as u64;

		info!(
			"Received response: {} {} ({}ms)",
			status, final_url, response_time
		);

		Ok(HttpResponseInfo {
			status,
			headers: response_headers,
			body,
			cookies: response_cookies,
			response_time_ms: response_time,
			final_url,
		})
	}

	pub async fn replay_request(&self, monitored_request: &crate::network::HttpRequest) -> Result<HttpResponseInfo> {
		let body = if monitored_request.body.is_empty() {
			None
		} else {
			Some(String::from_utf8_lossy(&monitored_request.body).to_string())
		};

		self.send_request(HttpRequestBuilder {
			method: monitored_request.method.clone(),
			url: monitored_request.url.clone(),
			headers: monitored_request.headers.clone(),
			body,
			timeout_seconds: 30,
			follow_redirects: true,
			verify_ssl: true,
		})
		.await
	}
}