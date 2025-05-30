use std::env;
use std::sync::Once;

static INIT: Once = Once::new();

/// Create a ureq agent with proxy configuration for ureq 2.x
// pub fn create_proxy_agent() -> Result<ureq::Agent, Box<dyn std::error::Error>> {
//     let proxy_url = "http://127.0.0.1:7897";

//     // Create proxy configuration for ureq 2.x
//     let proxy = ureq::Proxy::new(proxy_url)?;

//     // Create agent with proxy using ureq 2.x API
//     let agent = ureq::AgentBuilder::new()
//         .proxy(proxy)
//         .build();

//     Ok(agent)
// }

/// Set proxy environment variables for ureq and other HTTP clients
pub fn setup_proxy() {
    let proxy_url = "http://127.0.0.1:7897";

    // Set environment variables that ureq and other HTTP clients recognize
    unsafe {
        env::set_var("HTTP_PROXY", proxy_url);
        env::set_var("HTTPS_PROXY", proxy_url);
        env::set_var("http_proxy", proxy_url);
        env::set_var("https_proxy", proxy_url);
        env::set_var("ALL_PROXY", proxy_url);
        env::set_var("all_proxy", proxy_url);

        // Some applications also check these
        env::set_var("HTTPS_PROXY_URL", proxy_url);
        env::set_var("HTTP_PROXY_URL", proxy_url);
    }

    // println!("Proxy configured: {}", proxy_url);
    // println!("Environment variables set:");
    // println!("  HTTP_PROXY: {}", env::var("HTTP_PROXY").unwrap_or_default());
    // println!("  HTTPS_PROXY: {}", env::var("HTTPS_PROXY").unwrap_or_default());
}

/// Check if proxy should be used based on environment variable
pub fn should_use_proxy() -> bool {
    env::var("HF_USE_PROXY").unwrap_or_else(|_| "true".to_string()) == "true"
}

/// Initialize proxy settings if needed (only once)
pub fn init_proxy() {
    INIT.call_once(|| {
        if should_use_proxy() {
            setup_proxy();
        }
    });
}
