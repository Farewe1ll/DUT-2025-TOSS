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
• Performance diagnostics for slow responses (6000ms+)
")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start network traffic monitoring and HTTP request parsing
    #[clap(long_about = "Monitor network packets on specified interface and parse HTTP requests. \
                         Requires administrator privileges. Supports BPF filters for packet filtering. \
                         Use --replay to enable automatic request replay functionality.")]
    Monitor {
        /// Network interface to monitor packets on (e.g., en0, eth0, wlan0)
        #[arg(short, long, default_value = "en0", help = "Network interface for packet monitoring")]
        interface: String,

        /// Berkeley Packet Filter (BPF) expression for packet filtering
        #[arg(short, long, default_value = "tcp port 80 or tcp port 443",
              help = "BPF filter expression (e.g., 'host example.com', 'tcp port 443')")]
        filter: String,

        /// Enable automatic replay of monitored HTTP requests
        #[arg(short, long, help = "Automatically replay monitored HTTP requests")]
        replay: bool,
    },

    /// Send HTTP/HTTPS requests with full customization support
    #[clap(long_about = "Send HTTP/HTTPS requests with custom methods, headers, and body content. \
                         Supports all standard HTTP methods (GET, POST, PUT, DELETE, PATCH, etc.). \
                         Automatically manages cookies and handles SSL/TLS verification. \
                         Includes timeout protection to prevent hanging requests.")]
    Request {
        /// HTTP method to use for the request
        #[arg(short, long, default_value = "GET",
              help = "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)")]
        method: String,

        /// Target URL for the HTTP request
        #[arg(short, long, help = "Target URL (http:// or https://)")]
        url: String,

        /// Custom HTTP headers in key:value format
        #[arg(short = 'H', long, help = "Custom headers (format: 'Name:Value')")]
        headers: Vec<String>,

        /// Request body content (JSON, XML, form data, etc.)
        #[arg(short, long, help = "Request body content")]
        body: Option<String>,

        /// Request timeout in seconds to prevent hanging
        #[arg(short, long, default_value = "30", help = "Request timeout in seconds")]
        timeout: u64,
    },

    /// Comprehensive cookie management with persistent storage
    #[clap(long_about = "Manage HTTP cookies with persistent JSON storage. \
                         Automatically handles cookie expiration and domain filtering. \
                         Stored cookies are automatically used in subsequent requests.")]
    Cookie {
        #[command(subcommand)]
        action: CookieAction,
    },

    /// View and analyze request logs with filtering and statistics
    #[clap(long_about = "View detailed request/response logs stored in JSON format. \
                         Supports filtering by source (monitor/manual/replay), content search, \
                         and comprehensive statistics generation.")]
    Logs {
        /// Maximum number of recent log entries to display
        #[arg(short, long, default_value = "10", help = "Number of recent logs to show")]
        limit: usize,

        /// Filter logs by request source type
        #[arg(short, long, help = "Filter by source: monitored, manual, or replay")]
        source: Option<String>,

        /// Search term to filter log entries
        #[arg(short, long, help = "Search query to filter logs")]
        query: Option<String>,

        /// Display comprehensive request statistics
        #[arg(long, help = "Show detailed statistics about requests")]
        stats: bool,
    },

    /// Replay previously monitord or manual HTTP requests
    #[clap(long_about = "Replay HTTP requests from the request log with customizable repetition and timing. \
                         Supports filtering by request source and batch processing with configurable delays. \
                         Useful for load testing and request pattern analysis.")]
    Replay {
        /// Number of recent requests to select for replay
        #[arg(short, long, default_value = "1", help = "Number of recent requests to replay")]
        limit: usize,

        /// Filter requests by source before replaying
        #[arg(short, long, help = "Filter by source: monitored or manual")]
        source: Option<String>,

        /// Number of times to replay each selected request
        #[arg(short, long, default_value = "1", help = "Repetition count for each request")]
        count: usize,

        /// Delay between replay requests in milliseconds
        #[arg(short, long, default_value = "100", help = "Delay between replays (ms)")]
        delay: u64,
    },

    /// Start HTTP/HTTPS proxy server with traffic interception
    #[clap(long_about = "Launch an HTTP/HTTPS proxy server that intercepts and logs traffic. \
                         Supports both HTTP requests and HTTPS CONNECT tunneling. \
                         All proxied requests are automatically logged for later analysis.")]
    Proxy {
        /// IP address to bind the proxy server to
        #[arg(short, long, default_value = "127.0.0.1", help = "Bind address (0.0.0.0 for all interfaces)")]
        address: String,

        /// Port number for the proxy server
        #[arg(short, long, default_value = "8080", help = "Port number for proxy server")]
        port: u16,
    },

    /// Advanced HTTP request performance analysis and diagnostics
    #[clap(long_about = "Comprehensive performance analysis tool for HTTP requests with intelligent diagnostics. \
                         Specializes in identifying 6000ms+ response time issues through multi-iteration testing. \
                         Provides detailed bottleneck analysis, performance classification, and optimization recommendations. \
                         Generates both console output and optional JSON reports.")]
    Analyze {
        /// Target URL for performance analysis
        #[arg(short, long, help = "URL to analyze for performance issues")]
        url: String,

        /// Number of test iterations for statistical accuracy
        #[arg(short, long, default_value = "5", help = "Number of test iterations (more = better accuracy)")]
        iterations: u32,

        /// Generate detailed JSON performance report
        #[arg(short, long, help = "Generate detailed JSON report file")]
        report: bool,
    },
}

#[derive(Subcommand)]
pub enum CookieAction {
    /// List stored cookies with optional domain filtering
    #[clap(long_about = "Display all stored cookies in a formatted table. \
                         Supports domain-based filtering to show cookies for specific websites.")]
    List {
        /// Filter cookies by domain name
        #[arg(short, long, help = "Filter cookies by domain (e.g., 'example.com')")]
        domain: Option<String>,
    },

    /// Manually add a cookie to the storage
    #[clap(long_about = "Add a cookie manually to the persistent storage. \
                         Useful for testing scenarios or importing cookies from other sources.")]
    Add {
        /// Cookie string in standard HTTP format
        #[arg(short, long, help = "Cookie string (e.g., 'name=value; Path=/; Domain=.example.com')")]
        cookie: String,

        /// Target URL to associate with the cookie
        #[arg(short, long, help = "Target URL for cookie context")]
        url: String,
    },

    /// Remove expired cookies from storage
    #[clap(long_about = "Clean up expired cookies from the persistent storage. \
                         Automatically removes cookies that have passed their expiration date.")]
    Clean,

    /// Clear all cookies from storage
    #[clap(long_about = "Remove all cookies from the persistent storage. \
                         This action cannot be undone - use with caution.")]
    Clear,
}

pub fn parse_headers(header_strings: Vec<String>) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    for header in header_strings {
        if let Some(colon_pos) = header.find(':') {
            let key = header[..colon_pos].trim().to_string();
            let value = header[colon_pos + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    headers
}
