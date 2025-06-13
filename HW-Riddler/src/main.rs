mod cli;
mod config;
mod cookie_manager;
mod network;
mod http_client;
mod logger;
mod performance_analyzer;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, CookieAction};
use config::Config;
use cookie_manager::CookieManager;
use http_client::{HttpClient, HttpRequestBuilder};
use logger::RequestLogger;
use network::{HttpParser, PacketMonitor};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();

	let log_level = cli.log_level.unwrap_or_else(|| "info".to_string());

	let env_filter = match EnvFilter::try_from_default_env() {
		Ok(filter) => filter,
		Err(_) => {
			EnvFilter::new(&log_level)
		}
	};

	fmt()
		.with_env_filter(env_filter)
		.with_level(true)
		.with_target(true)
		.pretty()
		.init();

	println!("Riddler 正在启动，日志级别: {}", log_level);
	info!("Starting Riddler with log level: {}", log_level);
	debug!("Debug logging enabled");

	let config = Config::default();


	let cookie_manager = Arc::new(CookieManager::new(config.storage.cookie_cache_path.clone()));
	let http_client = Arc::new(HttpClient::new(cookie_manager.clone())?);
	let logger = Arc::new(RequestLogger::new(&config.storage.request_log_path).await?);


	if let Err(e) = cookie_manager.load_from_file().await {
		warn!("Failed to load cookies from file: {}", e);
	}

	match cli.command {
		Commands::Monitor { interface, filter, replay } => {
			start_monitor(interface, filter, replay, cookie_manager.clone(), http_client.clone(), logger.clone()).await?;
		}

		Commands::Request { method, url, headers, body, timeout } => {
			send_manual_request(method, url, headers, body, timeout, http_client.clone(), logger.clone()).await?;
		}

		Commands::Cookie { action } => {
			handle_cookie_command(action, cookie_manager.clone()).await?;
		}

		Commands::Logs { limit, source, query, stats, path } => {
			if let Some(ref custom_path) = path {
				println!("使用自定义日志文件: {}", custom_path);
				let custom_logger = Arc::new(RequestLogger::new(custom_path).await?);
				show_logs(limit, source, query, stats, custom_logger).await?;
			} else {
				println!("使用默认日志文件: {}", config.storage.request_log_path);
				show_logs(limit, source, query, stats, logger.clone()).await?;
			}
		}

		Commands::Replay { limit, source, count, delay } => {
			replay_requests(limit, source, count, delay, http_client.clone(), logger.clone()).await?;
		}

		Commands::Proxy { address, port } => {
			start_proxy(address, port).await?;
		}

		Commands::Analyze { url, iterations, report } => {
			analyze_performance(url, iterations, report, http_client.clone()).await?;
		}
	}


	if let Err(e) = cookie_manager.save_to_file().await {
		error!("Failed to save cookies: {}", e);
	}

	Ok(())
}

async fn start_monitor(
	interface: String,
	filter: String,
	replay: bool,
	_cookie_manager: Arc<CookieManager>,
	http_client: Arc<HttpClient>,
	logger: Arc<RequestLogger>,
) -> Result<()> {
	if interface.starts_with("<请用") {
		eprintln!("错误: 未指定网络接口。请使用--interface参数指定有效的网络接口。");
		println!("可用网络接口列表:");

		for (i, device) in config::list_available_interfaces().iter().enumerate() {
			println!("  {}: {}", i+1, device);
		}

		return Err(anyhow::anyhow!("未指定有效网络接口"));
	}

	info!("Starting network monitor on {} with filter: {}", interface, filter);
	debug!("Initializing packet monitor with detailed logging");

	#[cfg(unix)]
	{
		if !cfg!(target_os = "macos") && unsafe { libc::geteuid() } != 0 {
			eprintln!("\n⚠️  警告: 在 Linux 上监控网络通常需要 root 权限！");
			eprintln!("请使用 sudo 运行此命令。\n");
		}
	}

	#[cfg(target_os = "windows")]
	if interface == "en0" {
		println!("注意: 在Windows上默认使用'en0'接口名称可能无效。建议使用--interface参数指定正确的接口名称。");
		println!("常见Windows网络接口名称通常是UUID格式，例如'\\Device\\NPF_{GUID}'");
		println!("请运行 'riddler monitor --help' 获取更多信息");
	}

	#[cfg(target_os = "linux")]
	if interface == "en0" {
		println!("注意: 在Linux上默认使用'en0'接口名称可能无效。建议使用--interface参数指定正确的接口名称。");
		println!("常见Linux网络接口名称: 'eth0', 'wlan0', 'ens33' 等。");
		println!("可以通过'ip link'命令查看系统上的可用接口");
	}

	let (packet_tx, mut packet_rx) = mpsc::unbounded_channel();
	let monitor = Arc::new(PacketMonitor::new(interface.clone(), filter.clone(), packet_tx));

	info!("Network monitor created, starting monitor...");

	if !config::interface_exists(&interface) {
		eprintln!("错误: 指定的网络接口 '{}' 不存在", interface);
		println!("可用网络接口列表:");
		for (i, device) in config::list_available_interfaces().iter().enumerate() {
			println!("  {}: {}", i+1, device);
		}
		return Err(anyhow::anyhow!("指定的网络接口不存在"));
	}

	if !config::validate_bpf_filter(&filter) {
		return Err(anyhow::anyhow!("无效的 BPF 过滤器语法: {}", filter));
	}

	let monitor_handle = match monitor.start_monitor().await {
		Ok(handle) => handle,
		Err(e) => {
			eprintln!("启动网络监控失败: {}", e);
			eprintln!("请检查:");
			eprintln!("  1. 是否以 root/管理员权限运行");
			eprintln!("  2. 指定的网络接口 '{}' 是否正确", interface);
			eprintln!("  3. 过滤器表达式 '{}' 是否有效", filter);
			return Err(e);
		}
	};

	println!("Packet monitor started.");
	println!("Ctrl + C then 'q' and Enter to quit");


	let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();
	let monitor_for_signal = monitor.clone();
	let monitor_for_keyboard = monitor.clone();
	let monitor_for_unix = monitor.clone();
	let shutdown_tx_clone = shutdown_tx.clone();
	let shutdown_tx_keyboard = shutdown_tx.clone();


	tokio::spawn(async move {
		use tokio::io::{AsyncBufReadExt, BufReader};

		let stdin = tokio::io::stdin();
		let reader = BufReader::new(stdin);
		let mut lines = reader.lines();

		loop {
			match lines.next_line().await {
				Ok(Some(line)) => {
					let input = line.trim().to_lowercase();
					if input == "q" || input == "quit" || input == "exit" {
						info!("User requested quit via keyboard input");
						monitor_for_keyboard.shutdown();
						monitor_for_keyboard.release_sender();
						let _ = shutdown_tx_keyboard.send(());
						break;
					} else if !input.is_empty() {
						println!("Unknown command '{}'. Press Ctrl + C then q and Enter to quit.", input);
					}
				}
				Ok(None) => {
					info!("Stdin closed, shutting down...");
					monitor_for_keyboard.shutdown();
					monitor_for_keyboard.release_sender();
					let _ = shutdown_tx_keyboard.send(());
					break;
				}
				Err(e) => {
					error!("Error reading from stdin: {}", e);
					break;
				}
			}
		}
	});


	tokio::spawn(async move {
		match tokio::signal::ctrl_c().await {
			Ok(()) => {
				info!("Ctrl+C received, shutting down");
				monitor_for_signal.shutdown();
				monitor_for_signal.release_sender();
				let _ = shutdown_tx.send(());


				tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
				std::process::exit(0);
			}
			Err(err) => {
				error!("Unable to listen for Ctrl+C signal: {}", err);
			}
		}
	});


	#[cfg(unix)]
	{
		tokio::spawn(async move {
			use tokio::signal::unix::{signal, SignalKind};

			let mut sigint = signal(SignalKind::interrupt()).expect("Failed to create SIGINT handler");
			let mut sigterm = signal(SignalKind::terminate()).expect("Failed to create SIGTERM handler");

			tokio::select! {
				_ = sigint.recv() => {
					info!("SIGINT received, shutting down");
					monitor_for_unix.shutdown();
					monitor_for_unix.release_sender();
					let _ = shutdown_tx_clone.send(());

					tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
					std::process::exit(0);
				}
				_ = sigterm.recv() => {
					info!("SIGTERM received, shutting down");
					monitor_for_unix.shutdown();
					monitor_for_unix.release_sender();
					let _ = shutdown_tx_clone.send(());

					tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
					std::process::exit(0);
				}
			}
		});
	}


	let _http_parser = network::HttpParser::new();
	let _http_request_count = 0;
	let mut _http_payload_packets = 0;
	let mut packet_count = 0;
	let mut exit_reason = "unknown";

	info!("HTTP监控已启动，等待捕获HTTP请求...");
	info!("如果没有看到任何网络包被捕获，请尝试生成一些HTTP流量 (例如访问 http://example.com)");

	println!("监控已启动。开始监听网络流量，日志将显示在这里...");
	debug!("Main loop starting, waiting for packets...");
	loop {

		match shutdown_rx.try_recv() {
			Ok(_) => {
				info!("Shutdown signal received, stopping monitor");
				exit_reason = "shutdown_signal";
				break;
			}
			Err(mpsc::error::TryRecvError::Disconnected) => {
				info!("Shutdown channel closed");
				exit_reason = "shutdown_channel_closed";
				break;
			}
			Err(mpsc::error::TryRecvError::Empty) => {

			}
		}


		if monitor_handle.is_finished() {
			info!("Monitor task completed");
			exit_reason = "monitor_task_finished";
			break;
		}


		let mut batch_processed = 0;
		const MAX_BATCH_SIZE: usize = 10;
		let mut channel_closed = false;

		while batch_processed < MAX_BATCH_SIZE {
			match packet_rx.try_recv() {
				Ok(packet) => {
					packet_count += 1;
					batch_processed += 1;

					debug!("Received packet #{} from {}:{}",
						packet_count, packet.src_ip, packet.src_port);

					if let Some(http_request) = HttpParser::parse_http_request(&packet) {
						info!("Monitored HTTP request #{}: {} {}", packet_count, http_request.method, http_request.url);


						if let Err(e) = logger.log_request(&http_request, "monitored").await {
							error!("Failed to log request: {}", e);
						}


						if replay {
							match http_client.replay_request(&http_request).await {
								Ok(response) => {
									info!("Replay response: {} - {}", response.status, response.final_url);


									if let Err(e) = logger.log_request_response(&http_request, &response, "replay").await {
										error!("Failed to log replay response: {}", e);
									}
								}
								Err(e) => {
									error!("Failed to replay request: {}", e);
								}
							}
						}
					} else {
						trace!("Packet #{} did not contain valid HTTP request", packet_count);
					}
				}
				Err(mpsc::error::TryRecvError::Empty) => {

					break;
				}
				Err(mpsc::error::TryRecvError::Disconnected) => {
					info!("Packet channel closed - monitor finished");
					channel_closed = true;
					exit_reason = "packet_channel_closed";
					break;
				}
			}
		}


		if channel_closed {
			break;
		}




		tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
	}

	info!("Main processing loop ended (reason: {})", exit_reason);


	if !monitor_handle.is_finished() {
		info!("Waiting for packet monitor task to finish...");
		if let Err(e) = monitor_handle.await {
			error!("Error waiting for monitor task: {}", e);
		}
	}

	info!("Monitored {} packets", packet_count);
	info!("Monitored {} packets total", packet_count);


	if exit_reason == "shutdown_signal" {
		std::process::exit(0);
	}

	Ok(())
}

async fn send_manual_request(
	method: String,
	url: String,
	headers: Vec<String>,
	body: Option<String>,
	timeout: u64,
	http_client: Arc<HttpClient>,
	logger: Arc<RequestLogger>,
) -> Result<()> {
	let parsed_headers = cli::parse_headers(headers);

	let request = HttpRequestBuilder {
		method: method.clone(),
		url: url.clone(),
		headers: parsed_headers.clone(),
		body: body.clone(),
		timeout_seconds: timeout,
		follow_redirects: true,
		verify_ssl: true,
	};

	info!("Sending {} request to {}", method, url);

	match http_client.send_request(request).await {
		Ok(response) => {
			println!("✅ Response Status: {}", response.status);
			println!("📝 Response Headers:");
			for (key, value) in &response.headers {
				println!("  {}: {}", key, value);
			}
			println!("📄 Response Body:");
			println!("{}", response.body);
			println!("⏱️  Response Time: {}ms", response.response_time_ms);

			if let Err(e) = logger.log_manual_request_response(
				&method,
				&url,
				parsed_headers,
				&body.clone().unwrap_or_default(),
				&response,
			).await {
				error!("Failed to log manual request: {}", e);
			}

			println!("✅ Request completed successfully!");
		}
		Err(e) => {
			error!("❌ Request failed: {}", e);
			println!("❌ Request failed: {}", e);
		}
	}

	Ok(())
}

async fn handle_cookie_command(
	action: CookieAction,
	cookie_manager: Arc<CookieManager>,
) -> Result<()> {
	match action {
		CookieAction::List { domain } => {
			let cookies = cookie_manager.list_cookies(domain.as_deref());
			for cookie in cookies {
				println!("{}={} (domain: {}, path: {})",
						cookie.name, cookie.value, cookie.domain, cookie.path);
			}
		}

		CookieAction::Add { cookie, url } => {
			let parsed_url = url::Url::parse(&url)?;
			cookie_manager.add_cookie(&parsed_url, &cookie)?;
			cookie_manager.save_to_file().await?;
			println!("Cookie added successfully");
		}

		CookieAction::Clean => {
			cookie_manager.clear_expired();
			cookie_manager.save_to_file().await?;
			println!("Expired cookies cleared");
		}

		CookieAction::Clear => {
			cookie_manager.clear_all();
			cookie_manager.save_to_file().await?;
			println!("All cookies cleared");
		}
	}

	Ok(())
}

async fn show_logs(
	limit: usize,
	source: Option<String>,
	query: Option<String>,
	show_stats: bool,
	logger: Arc<RequestLogger>,
) -> Result<()> {
	if show_stats {
		let stats = logger.get_request_stats().await?;
		println!("=== Request Statistics ===");
		println!("Total Requests: {}", stats.total_requests);
		println!("Monitored: {}, Manual: {}, Replay: {}",
				stats.monitored_requests, stats.manual_requests, stats.replay_requests);
		println!("Successful: {}, Failed: {}", stats.successful_requests, stats.failed_requests);
		println!("Average Response Time: {}ms", stats.average_response_time);

		println!("\nMethods:");
		for (method, count) in &stats.methods {
			println!("  {}: {}", method, count);
		}
		println!();
		return Ok(());
	}

	let logs = if let Some(search_query) = query {
		logger.search_logs(&search_query, limit).await?
	} else {
		logger.read_recent_logs(limit).await?
	};

	for log in logs {
		if let Some(ref filter_source) = source {
			if log.source != *filter_source {
				continue;
			}
		}

		println!("=== {} [{}] ===", log.timestamp, log.source);
		println!("{} {} ({}:{})",
				log.request.method,
				log.request.url,
				log.request.source_ip,
				log.request.source_port);

		if !log.request.body_preview.is_empty() {
			println!("Body Preview: {}", log.request.body_preview);
		}

		if let Some(ref response) = log.response {
			println!("Response: {} ({}ms)", response.status, response.response_time_ms);
		}
		println!();
	}

	Ok(())
}

async fn start_proxy(address: String, port: u16) -> Result<()> {
	println!("Starting HTTP/HTTPS proxy server on {}:{}", address, port);

	use tokio::net::TcpListener;

	let listener = TcpListener::bind(format!("{}:{}", address, port)).await?;
	info!("Proxy server listening on {}:{}", address, port);

	loop {
		let (stream, addr) = listener.accept().await?;
		info!("New connection from: {}", addr);

		tokio::spawn(async move {
			if let Err(e) = handle_proxy_connection(stream).await {
				error!("Proxy connection error: {}", e);
			}
		});
	}
}

async fn handle_proxy_connection(mut stream: tokio::net::TcpStream) -> Result<()> {
	use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
	use tokio::net::TcpStream;

	let mut reader = BufReader::new(&mut stream);
	let mut request_line = String::new();
	reader.read_line(&mut request_line).await?;

	let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
	if parts.len() < 2 {
		return Ok(());
	}

	let method = parts[0];
	let target = parts[1];

	if method == "CONNECT" {

		let host_port: Vec<&str> = target.split(':').collect();
		if host_port.len() != 2 {
			return Ok(());
		}

		let host = host_port[0];
		let port: u16 = host_port[1].parse().unwrap_or(443);

		info!("CONNECT request to {}:{}", host, port);


		match TcpStream::connect(format!("{}:{}", host, port)).await {
			Ok(target_stream) => {

				let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
				stream.write_all(response.as_bytes()).await?;


				let (mut client_read, mut client_write) = stream.into_split();
				let (mut target_read, mut target_write) = target_stream.into_split();

				tokio::spawn(async move {
					let _ = tokio::io::copy(&mut client_read, &mut target_write).await;
				});

				tokio::spawn(async move {
					let _ = tokio::io::copy(&mut target_read, &mut client_write).await;
				});
			}
			Err(e) => {
				error!("Failed to connect to target: {}", e);
				let response = "HTTP/1.1 502 Bad Gateway\r\n\r\n";
				stream.write_all(response.as_bytes()).await?;
			}
		}
	} else {

		info!("HTTP request: {} {}", method, target);


		let mut headers = Vec::new();
		loop {
			let mut line = String::new();
			reader.read_line(&mut line).await?;
			if line.trim().is_empty() {
				break;
			}
			headers.push(line);
		}


		let response = format!(
			"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 27\r\n\r\nProxy handled {} request",
			method
		);
		stream.write_all(response.as_bytes()).await?;
	}

	Ok(())
}

async fn replay_requests(
	limit: usize,
	source: Option<String>,
	count: usize,
	delay: u64,
	http_client: Arc<HttpClient>,
	logger: Arc<RequestLogger>,
) -> Result<()> {
	info!("Starting request replay - limit: {}, count: {}, delay: {}ms", limit, count, delay);


	let logs = logger.read_recent_logs(limit).await?;
	let mut requests_to_replay = Vec::new();

	for log in logs {

		if let Some(ref filter_source) = source {
			if log.source != *filter_source {
				continue;
			}
		}


		let request = HttpRequestBuilder {
			method: log.request.method.clone(),
			url: log.request.url.clone(),
			headers: log.request.headers.clone(),
			body: if log.request.body_preview.is_empty() {
				None
			} else {
				Some(log.request.body_preview.clone())
			},
			timeout_seconds: 30,
			follow_redirects: true,
			verify_ssl: true,
		};

		requests_to_replay.push(request);
	}

	if requests_to_replay.is_empty() {
		println!("No requests found to replay");
		return Ok(());
	}

	println!("Found {} requests to replay", requests_to_replay.len());


	for (i, request) in requests_to_replay.iter().enumerate() {
		println!("\n=== Replaying Request {} ===", i + 1);
		println!("{} {}", request.method, request.url);

		for replay_num in 1..=count {
			println!("Replay {}/{}", replay_num, count);

			match http_client.send_request(request.clone()).await {
				Ok(response) => {
					println!("✅ Response: {} ({}ms)", response.status, response.response_time_ms);


					// 直接在主流程中记录日志，不使用tokio::spawn
					if let Err(e) = logger.log_replay_request_response(&request, &response).await {
						error!("Failed to log replay: {}", e);
					}
				}
				Err(e) => {
					println!("❌ Error: {}", e);
				}
			}


			if replay_num < count && delay > 0 {
				tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
			}
		}


		if i < requests_to_replay.len() - 1 && delay > 0 {
			tokio::time::sleep(tokio::time::Duration::from_millis(delay * 2)).await;
		}
	}

	println!("\n✓ Replay completed!");
	Ok(())
}

async fn analyze_performance(
	url: String,
	iterations: u32,
	generate_report: bool,
	http_client: Arc<HttpClient>,
) -> Result<()> {
	use performance_analyzer::PerformanceAnalyzer;

	println!("🔍 Starting performance analysis for: {}", url);
	println!("📊 Running {} test iterations...\n", iterations);

	let analyzer = PerformanceAnalyzer::new(http_client);


	match analyzer.run_performance_test(&url, iterations).await {
		Ok(analyses) => {
			if analyses.is_empty() {
				println!("❌ No successful requests completed");
				return Ok(());
			}


			for (i, analysis) in analyses.iter().enumerate() {
				println!("=== Test {} Results ===", i + 1);
				println!("Response Time: {}ms", analysis.metrics.total_time_ms);
				println!("Status: HTTP {}",
					match analysis.severity {
						performance_analyzer::PerformanceSeverity::Excellent => "✅ Excellent",
						performance_analyzer::PerformanceSeverity::Good => "✅ Good",
						performance_analyzer::PerformanceSeverity::Average => "⚠️ Average",
						performance_analyzer::PerformanceSeverity::Poor => "⚠️ Poor",
						performance_analyzer::PerformanceSeverity::Critical => "❌ Critical",
					}
				);

				if analysis.metrics.total_time_ms > 6000 {
					println!("🚨 CRITICAL: Response time exceeded 6 seconds!");
				}
				println!();
			}


			let summary = analyzer.generate_summary_report(&analyses);
			println!("{}", summary);

			if generate_report {

				let report_path = "performance_report.json";
				match tokio::fs::write(
					report_path,
					serde_json::to_string_pretty(&analyses)?
				).await {
					Ok(_) => println!("📄 Detailed report saved to: {}", report_path),
					Err(e) => println!("⚠️ Failed to save report: {}", e),
				}
			}


			let slow_requests: Vec<_> = analyses.iter()
				.filter(|a| a.metrics.total_time_ms > 6000)
				.collect();

			if !slow_requests.is_empty() {
				println!("\n🔍 ANALYSIS OF 6000ms+ RESPONSE TIMES:");
				println!("Found {} requests with critical response times", slow_requests.len());

				for analysis in slow_requests {
					println!("\n{}", analysis.analysis);
					println!("Recommendations:");
					for rec in &analysis.recommendations {
						println!("• {}", rec);
					}
				}

				println!("\n📋 COMMON FACTORS CAUSING 6000ms+ RESPONSE TIMES:");
				println!("1. 🌐 Network Latency Issues:");
				println!("   - High RTT (Round Trip Time) to target server");
				println!("   - Geographic distance to server location");
				println!("   - Network congestion or packet loss");

				println!("2. 🖥️ Server-Side Performance:");
				println!("   - Server overload or high resource utilization");
				println!("   - Slow database queries or backend processing");
				println!("   - Insufficient server capacity");

				println!("3. 🔗 Connection Issues:");
				println!("   - DNS resolution delays");
				println!("   - TCP connection establishment overhead");
				println!("   - TLS handshake delays");

				println!("4. 🚦 ISP or Infrastructure:");
				println!("   - Internet Service Provider throttling");
				println!("   - Routing inefficiencies");
				println!("   - CDN or proxy server delays");

				println!("5. 📦 Data Transfer:");
				println!("   - Large response payloads");
				println!("   - Lack of compression (gzip/brotli)");
				println!("   - Inefficient data serialization");
			}
		}
		Err(e) => {
			println!("❌ Performance analysis failed: {}", e);
		}
	}

	Ok(())
}