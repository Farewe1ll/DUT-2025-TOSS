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
use network::{HttpParser, PacketCapture};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let config = Config::default();

    // Initialize components
    let cookie_manager = Arc::new(CookieManager::new(config.storage.cookie_cache_path.clone()));
    let http_client = Arc::new(HttpClient::new(cookie_manager.clone())?);
    let logger = Arc::new(RequestLogger::new(&config.storage.request_log_path).await?);

    // Load existing cookies
    if let Err(e) = cookie_manager.load_from_file().await {
        warn!("Failed to load cookies from file: {}", e);
    }

    match cli.command {
        Commands::Capture { interface, filter, replay } => {
            start_capture(interface, filter, replay, cookie_manager.clone(), http_client.clone(), logger.clone()).await?;
        }

        Commands::Request { method, url, headers, body, timeout } => {
            send_manual_request(method, url, headers, body, timeout, http_client.clone(), logger.clone()).await?;
        }

        Commands::Cookie { action } => {
            handle_cookie_command(action, cookie_manager.clone()).await?;
        }

        Commands::Logs { limit, source, query, stats } => {
            show_logs(limit, source, query, stats, logger.clone()).await?;
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

    // Save cookies before exit
    if let Err(e) = cookie_manager.save_to_file().await {
        error!("Failed to save cookies: {}", e);
    }

    Ok(())
}

async fn start_capture(
    interface: String,
    filter: String,
    replay: bool,
    _cookie_manager: Arc<CookieManager>,
    http_client: Arc<HttpClient>,
    logger: Arc<RequestLogger>,
) -> Result<()> {
    info!("Starting network capture on {} with filter: {}", interface, filter);

    let (packet_tx, mut packet_rx) = mpsc::unbounded_channel();
    let capture = PacketCapture::new(interface, filter, packet_tx);

    // Start packet capture
    capture.start_capture().await?;

    println!("Packet capture started. Press Ctrl+C to stop.");

    // Setup signal handler
    let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();

    #[cfg(unix)]
    {
        use tokio::signal;
        tokio::spawn(async move {
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();

            tokio::select! {
                _ = sigint.recv() => {
                    info!("Received SIGINT");
                }
                _ = sigterm.recv() => {
                    info!("Received SIGTERM");
                }
            }

            let _ = shutdown_tx.send(());
        });
    }

    let mut packet_count = 0;

    // Process captured packets
    loop {
        tokio::select! {
            packet = packet_rx.recv() => {
                match packet {
                    Some(packet) => {
                        packet_count += 1;

                        if let Some(http_request) = HttpParser::parse_http_request(&packet) {
                            info!("Captured HTTP request #{}: {} {}", packet_count, http_request.method, http_request.url);

                            // Log the captured request
                            if let Err(e) = logger.log_request(&http_request, "captured").await {
                                error!("Failed to log request: {}", e);
                            }

                            // Replay request if enabled
                            if replay {
                                match http_client.replay_request(&http_request).await {
                                    Ok(response) => {
                                        info!("Replay response: {} - {}", response.status, response.final_url);

                                        // Log the replayed request with response
                                        if let Err(e) = logger.log_request_response(&http_request, &response, "replay").await {
                                            error!("Failed to log replay response: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to replay request: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        info!("Packet channel closed");
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received, stopping capture");
                break;
            }
        }
    }

    info!("Captured {} packets total", packet_count);
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

            // Log the manual request (async operation)
            tokio::spawn({
                let logger = logger.clone();
                let method = method.clone();
                let url = url.clone();
                let parsed_headers = parsed_headers.clone();
                let body = body.clone().unwrap_or_default();
                let response = response.clone();

                async move {
                    if let Err(e) = logger.log_manual_request_response(
                        &method,
                        &url,
                        parsed_headers,
                        &body,
                        &response,
                    ).await {
                        error!("Failed to log manual request: {}", e);
                    }
                }
            });

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
        println!("Captured: {}, Manual: {}, Replay: {}",
                 stats.captured_requests, stats.manual_requests, stats.replay_requests);
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
        // Handle HTTPS CONNECT request
        let host_port: Vec<&str> = target.split(':').collect();
        if host_port.len() != 2 {
            return Ok(());
        }

        let host = host_port[0];
        let port: u16 = host_port[1].parse().unwrap_or(443);

        info!("CONNECT request to {}:{}", host, port);

        // Connect to target server
        match TcpStream::connect(format!("{}:{}", host, port)).await {
            Ok(target_stream) => {
                // Send 200 Connection Established
                let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
                stream.write_all(response.as_bytes()).await?;

                // Start tunneling
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
        // Handle regular HTTP request
        info!("HTTP request: {} {}", method, target);

        // Read remaining headers
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            if line.trim().is_empty() {
                break;
            }
            headers.push(line);
        }

        // For demonstration, just return a simple response
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

    // Get recent requests to replay
    let logs = logger.read_recent_logs(limit).await?;
    let mut requests_to_replay = Vec::new();

    for log in logs {
        // Filter by source if specified
        if let Some(ref filter_source) = source {
            if log.source != *filter_source {
                continue;
            }
        }

        // Convert log entry back to HttpRequestBuilder
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

    // Replay each request `count` times
    for (i, request) in requests_to_replay.iter().enumerate() {
        println!("\n=== Replaying Request {} ===", i + 1);
        println!("{} {}", request.method, request.url);

        for replay_num in 1..=count {
            println!("Replay {}/{}", replay_num, count);

            match http_client.send_request(request.clone()).await {
                Ok(response) => {
                    println!("✅ Response: {} ({}ms)", response.status, response.response_time_ms);

                    // Log the replayed request (async operation)
                    tokio::spawn({
                        let logger = logger.clone();
                        let request = request.clone();
                        let response = response.clone();

                        async move {
                            if let Err(e) = logger.log_replay_request_response(&request, &response).await {
                                error!("Failed to log replay: {}", e);
                            }
                        }
                    });
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                }
            }

            // Add delay between replays
            if replay_num < count && delay > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
        }

        // Add delay between different requests
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

    // Run performance test
    match analyzer.run_performance_test(&url, iterations).await {
        Ok(analyses) => {
            if analyses.is_empty() {
                println!("❌ No successful requests completed");
                return Ok(());
            }

            // Show individual results
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

            // Generate summary report
            let summary = analyzer.generate_summary_report(&analyses);
            println!("{}", summary);

            if generate_report {
                // Save detailed report to file
                let report_path = "performance_report.json";
                match tokio::fs::write(
                    report_path,
                    serde_json::to_string_pretty(&analyses)?
                ).await {
                    Ok(_) => println!("📄 Detailed report saved to: {}", report_path),
                    Err(e) => println!("⚠️ Failed to save report: {}", e),
                }
            }

            // Provide specific analysis for 6000ms+ response times
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
