use anyhow::Result;
use cookie_store::Cookie;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieEntry {
	pub name: String,
	pub value: String,
	pub domain: String,
	pub path: String,
	pub expires: Option<u64>,
	pub secure: bool,
	pub http_only: bool,
	pub same_site: Option<String>,
}

#[derive(Debug)]
pub struct CookieManager {
	store: Arc<DashMap<String, CookieEntry>>,
	file_path: String,
}

impl CookieManager {
	pub fn new(file_path: String) -> Self {
		Self {
			store: Arc::new(DashMap::new()),
			file_path,
		}
	}

	pub async fn load_from_file(&self) -> Result<()> {
		if let Ok(content) = fs::read_to_string(&self.file_path).await {
			if let Ok(cookies) = serde_json::from_str::<Vec<CookieEntry>>(&content) {
				for cookie in cookies {
					let key = format!("{}:{}", cookie.domain, cookie.name);
					self.store.insert(key, cookie);
				}
			}
		}
		Ok(())
	}

	pub async fn save_to_file(&self) -> Result<()> {
		let cookies: Vec<CookieEntry> = self.store.iter().map(|entry| entry.value().clone()).collect();
		let content = serde_json::to_string_pretty(&cookies)?;
		fs::write(&self.file_path, content).await?;
		Ok(())
	}

	pub fn add_cookie(&self, url: &Url, cookie_str: &str) -> Result<()> {
		if let Ok(cookie) = Cookie::parse(cookie_str, url) {
			let entry = CookieEntry {
				name: cookie.name().to_string(),
				value: cookie.value().to_string(),
				domain: cookie.domain().unwrap_or("").to_string(),
				path: cookie.path().unwrap_or("/").to_string(),
				expires: cookie.expires_datetime().map(|dt| {
					dt.unix_timestamp() as u64
				}),
				secure: cookie.secure().unwrap_or(false),
				http_only: cookie.http_only().unwrap_or(false),
				same_site: cookie.same_site().map(|s| format!("{:?}", s)),
			};

			let key = format!("{}:{}", entry.domain, entry.name);
			self.store.insert(key, entry);
		}
		Ok(())
	}

	pub fn get_cookies_for_url(&self, url: &Url) -> Vec<String> {
		let domain = url.domain().unwrap_or("");
		let path = url.path();
		let is_secure = url.scheme() == "https";

		let now = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_secs();

		self.store
			.iter()
			.filter_map(|entry| {
				let cookie = entry.value();

				let domain_match = domain == cookie.domain ||
					(cookie.domain.starts_with(".") && domain.ends_with(&cookie.domain[1..]));

				if !domain_match {
					return None;
				}

				if !path.starts_with(&cookie.path) {
					return None;
				}

				if let Some(expires) = cookie.expires {
					if now > expires {
						return None;
					}
				}

				if cookie.secure && !is_secure {
					return None;
				}

				Some(format!("{}={}", cookie.name, cookie.value))
			})
			.collect()
	}

	pub fn clear_expired(&self) {
		let now = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_secs();

		self.store.retain(|_, cookie| {
			if let Some(expires) = cookie.expires {
				now <= expires
			} else {
				true
			}
		});
	}

	pub fn list_cookies(&self, domain_filter: Option<&str>) -> Vec<CookieEntry> {
		self.store
			.iter()
			.filter_map(|entry| {
				let cookie = entry.value();
				if let Some(domain) = domain_filter {
					if !cookie.domain.contains(domain) {
						return None;
					}
				}
				Some(cookie.clone())
			})
			.collect()
	}

	pub fn clear_all(&self) {
		self.store.clear();
	}
}