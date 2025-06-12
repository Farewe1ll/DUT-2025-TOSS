use clap::{Parser, Subcommand};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "riddler")]
#[command(about = "HW-Riddler - A comprehensive network traffic monitor, HTTP/HTTPS request tool, and performance analyzer")]
#[command(long_about = "
HW-Riddler is a powerful network analysis tool that provides:
• Network packet monitor and HTTP request monitoring
• HTTP/HTTPS client with cookie management
• Request logging, replay, and performance analysis
• Proxy server functionality
• Performance diagnostics
")]
#[command(version)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Commands,

	#[arg(long, help = "Set log level (error, warn, info, debug, trace)", default_value = "info")]
	pub log_level: Option<String>,

	#[arg(long, help = "Show verbose network traffic (all packets)")]
	pub verbose_network: bool,
}

#[derive(Subcommand)]
pub enum Commands {
	#[clap(long_about = "Monitor network packets on specified interface and parse HTTP requests. \
						Requires administrator privileges. Supports BPF filters for packet filtering. \
						Use --replay to enable automatic request replay functionality.")]
	Monitor {
		#[arg(short, long, default_value = "en0", help = "Network interface for packet monitoring")]
		interface: String,

		#[arg(short, long, default_value = "tcp port 80 or tcp port 443",
			help = "BPF filter expression (e.g., 'host example.com', 'tcp port 443')")]
		filter: String,

		#[arg(short, long, help = "Automatically replay monitored HTTP requests")]
		replay: bool,
	},

	#[clap(long_about = "Send HTTP/HTTPS requests with custom methods, headers, and body content. \
						Supports all standard HTTP methods (GET, POST, PUT, DELETE, PATCH, etc.). \
						Automatically manages cookies and handles SSL/TLS verification. \
						Includes timeout protection to prevent hanging requests.")]
	Request {
		#[arg(short, long, default_value = "GET",
			help = "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)")]
		method: String,

		#[arg(short, long, help = "Target URL (http:// or https://)")]
		url: String,

		#[arg(short = 'H', long, help = "Custom headers (format: 'Name:Value')")]
		headers: Vec<String>,

		#[arg(short, long, help = "Request body content")]
		body: Option<String>,

		#[arg(short, long, default_value = "30", help = "Request timeout in seconds")]
		timeout: u64,
	},

	#[clap(long_about = "Manage HTTP cookies with persistent JSON storage. \
						Automatically handles cookie expiration and domain filtering. \
						Stored cookies are automatically used in subsequent requests.")]
	Cookie {
		#[command(subcommand)]
		action: CookieAction,
	},

	#[clap(long_about = "View detailed request/response logs stored in JSON format. \
						Supports filtering by source (monitor/manual/replay), content search, \
						and comprehensive statistics generation.")]
	Logs {
		#[arg(short, long, default_value = "10", help = "Number of recent logs to show")]
		limit: usize,

		#[arg(short, long, help = "Filter by source: monitored, manual, or replay")]
		source: Option<String>,

		#[arg(short, long, help = "Search query to filter logs")]
		query: Option<String>,

		#[arg(long, help = "Show detailed statistics about requests")]
		stats: bool,

		#[arg(short = 'p', long, help = "Specify custom log file path (overrides config setting)")]
		path: Option<String>,
	},

	#[clap(long_about = "Replay HTTP requests from the request log with customizable repetition and timing. \
						Supports filtering by request source and batch processing with configurable delays. \
						Useful for load testing and request pattern analysis.")]
	Replay {
		#[arg(short, long, default_value = "1", help = "Number of recent requests to replay")]
		limit: usize,

		#[arg(short, long, help = "Filter by source: monitored or manual")]
		source: Option<String>,

		#[arg(short, long, default_value = "1", help = "Repetition count for each request")]
		count: usize,

		#[arg(short, long, default_value = "100", help = "Delay between replays (ms)")]
		delay: u64,
	},

	#[clap(long_about = "Launch an HTTP/HTTPS proxy server that intercepts and logs traffic. \
						Supports both HTTP requests and HTTPS CONNECT tunneling. \
						All proxied requests are automatically logged for later analysis.")]
	Proxy {
		#[arg(short, long, default_value = "127.0.0.1", help = "Bind address (0.0.0.0 for all interfaces)")]
		address: String,

		#[arg(short, long, default_value = "8080", help = "Port number for proxy server")]
		port: u16,
	},

	#[clap(long_about = "Comprehensive performance analysis tool for HTTP requests with intelligent diagnostics. \
						Specializes in identifying 6000ms+ response time issues through multi-iteration testing. \
						Provides detailed bottleneck analysis, performance classification, and optimization recommendations. \
						Generates both console output and optional JSON reports.")]
	Analyze {
		#[arg(short, long, help = "URL to analyze for performance issues")]
		url: String,

		#[arg(short, long, default_value = "5", help = "Number of test iterations (more = better accuracy)")]
		iterations: u32,

		#[arg(short, long, help = "Generate detailed JSON report file")]
		report: bool,
	},
}

#[derive(Subcommand)]
pub enum CookieAction {
	#[clap(long_about = "Display all stored cookies in a formatted table. \
						Supports domain-based filtering to show cookies for specific websites.")]
	List {
		#[arg(short, long, help = "Filter cookies by domain (e.g., 'example.com')")]
		domain: Option<String>,
	},

	#[clap(long_about = "Add a cookie manually to the persistent storage. \
						Useful for testing scenarios or importing cookies from other sources.")]
	Add {
		#[arg(short, long, help = "Cookie string (e.g., 'name=value; Path=/; Domain=.example.com')")]
		cookie: String,

		#[arg(short, long, help = "Target URL for cookie context")]
		url: String,
	},

	#[clap(long_about = "Clean up expired cookies from the persistent storage. \
						Automatically removes cookies that have passed their expiration date.")]
	Clean,

	#[clap(long_about = "Remove all cookies from the persistent storage. \
						This action cannot be undone - use with caution.")]
	Clear,
}

pub fn parse_headers(header_strings: Vec<String>) -> HashMap<String, String> {
	header_strings.into_iter()
		.filter_map(|header| {
			let parts: Vec<&str> = header.splitn(2, ':').collect();
			if parts.len() == 2 {
				Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
			} else {
				None
			}
		})
		.collect()
}