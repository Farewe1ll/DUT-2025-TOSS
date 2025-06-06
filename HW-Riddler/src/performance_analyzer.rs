use crate::http_client::{HttpClient, HttpRequestBuilder, HttpResponseInfo};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub dns_resolution_ms: Option<u64>,
    pub tcp_connect_ms: Option<u64>,
    pub tls_handshake_ms: Option<u64>,
    pub request_send_ms: u64,
    pub first_byte_ms: u64,
    pub response_download_ms: u64,
    pub total_time_ms: u64,
    pub response_size_bytes: usize,
    pub network_conditions: NetworkConditions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConditions {
    pub estimated_bandwidth_mbps: Option<f64>,
    pub latency_factors: Vec<String>,
    pub performance_bottlenecks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysis {
    pub url: String,
    pub metrics: PerformanceMetrics,
    pub analysis: String,
    pub recommendations: Vec<String>,
    pub severity: PerformanceSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceSeverity {
    Excellent,  // < 100ms
    Good,       // 100-500ms
    Average,    // 500-1000ms
    Poor,       // 1000-3000ms
    Critical,   // > 3000ms
}

pub struct PerformanceAnalyzer {
    http_client: Arc<HttpClient>,
}

impl PerformanceAnalyzer {
    pub fn new(http_client: Arc<HttpClient>) -> Self {
        Self { http_client }
    }

    pub async fn analyze_request(&self, request: &HttpRequestBuilder) -> Result<PerformanceAnalysis> {
        info!("Starting performance analysis for: {}", request.url);

        let overall_start = Instant::now();

        // Perform the request with detailed timing
        let response = self.http_client.send_request(request.clone()).await?;

        let total_time = overall_start.elapsed().as_millis() as u64;

        // Analyze the results
        let metrics = self.build_metrics(&response, total_time);
        let analysis = self.generate_analysis(&metrics, &response);
        let recommendations = self.generate_recommendations(&metrics);
        let severity = self.determine_severity(total_time);

        Ok(PerformanceAnalysis {
            url: request.url.clone(),
            metrics,
            analysis,
            recommendations,
            severity,
        })
    }

    fn build_metrics(&self, response: &HttpResponseInfo, total_time: u64) -> PerformanceMetrics {
        let response_size = response.body.len();

        // Estimate bandwidth based on response size and time
        let estimated_bandwidth = if total_time > 0 {
            let bytes_per_second = (response_size as f64 * 1000.0) / total_time as f64;
            Some((bytes_per_second * 8.0) / 1_000_000.0) // Convert to Mbps
        } else {
            None
        };

        // Identify latency factors
        let mut latency_factors = Vec::new();
        let mut bottlenecks = Vec::new();

        if total_time > 6000 {
            latency_factors.push("Extremely high response time detected".to_string());
            bottlenecks.push("Network congestion or server overload".to_string());
        }

        if total_time > 3000 {
            latency_factors.push("High latency connection".to_string());
            bottlenecks.push("Slow server processing or network issues".to_string());
        }

        if response_size > 1_000_000 {
            latency_factors.push("Large response payload".to_string());
            bottlenecks.push("Data transfer time".to_string());
        }

        // Analyze response headers for caching indicators
        if !response.headers.contains_key("cache-control") {
            latency_factors.push("No cache headers".to_string());
        }

        if response.headers.get("server").map_or(false, |s| s.contains("nginx")) {
            latency_factors.push("Nginx server detected".to_string());
        }

        PerformanceMetrics {
            dns_resolution_ms: None, // Would need custom implementation to measure
            tcp_connect_ms: None,    // Would need custom implementation to measure
            tls_handshake_ms: None,  // Would need custom implementation to measure
            request_send_ms: 0,      // Approximated
            first_byte_ms: total_time / 3, // Estimated
            response_download_ms: total_time * 2 / 3, // Estimated
            total_time_ms: total_time,
            response_size_bytes: response_size,
            network_conditions: NetworkConditions {
                estimated_bandwidth_mbps: estimated_bandwidth,
                latency_factors,
                performance_bottlenecks: bottlenecks,
            },
        }
    }

    fn generate_analysis(&self, metrics: &PerformanceMetrics, response: &HttpResponseInfo) -> String {
        let mut analysis = format!(
            "Performance Analysis for HTTP {} response:\n\n",
            response.status
        );

        analysis.push_str(&format!(
            "• Total Response Time: {}ms\n",
            metrics.total_time_ms
        ));

        analysis.push_str(&format!(
            "• Response Size: {} bytes ({:.2} KB)\n",
            metrics.response_size_bytes,
            metrics.response_size_bytes as f64 / 1024.0
        ));

        if let Some(bandwidth) = metrics.network_conditions.estimated_bandwidth_mbps {
            analysis.push_str(&format!(
                "• Estimated Bandwidth: {:.2} Mbps\n",
                bandwidth
            ));
        }

        if metrics.total_time_ms > 6000 {
            analysis.push_str("\n⚠️  CRITICAL PERFORMANCE ISSUE DETECTED:\n");
            analysis.push_str("Response time exceeds 6 seconds, indicating severe performance problems.\n");
        } else if metrics.total_time_ms > 3000 {
            analysis.push_str("\n⚠️  Performance Warning:\n");
            analysis.push_str("Response time exceeds 3 seconds, indicating performance issues.\n");
        }

        if !metrics.network_conditions.latency_factors.is_empty() {
            analysis.push_str("\nIdentified Latency Factors:\n");
            for factor in &metrics.network_conditions.latency_factors {
                analysis.push_str(&format!("• {}\n", factor));
            }
        }

        if !metrics.network_conditions.performance_bottlenecks.is_empty() {
            analysis.push_str("\nPerformance Bottlenecks:\n");
            for bottleneck in &metrics.network_conditions.performance_bottlenecks {
                analysis.push_str(&format!("• {}\n", bottleneck));
            }
        }

        analysis
    }

    fn generate_recommendations(&self, metrics: &PerformanceMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        if metrics.total_time_ms > 6000 {
            recommendations.push("Critical: Investigate server health and network connectivity".to_string());
            recommendations.push("Consider implementing request timeouts shorter than 6 seconds".to_string());
            recommendations.push("Check for DNS resolution issues".to_string());
            recommendations.push("Verify target server is responding properly".to_string());
        }

        if metrics.total_time_ms > 3000 {
            recommendations.push("Implement connection pooling to reduce connection overhead".to_string());
            recommendations.push("Consider using HTTP/2 or HTTP/3 for better performance".to_string());
            recommendations.push("Add response caching where appropriate".to_string());
        }

        if metrics.response_size_bytes > 1_000_000 {
            recommendations.push("Consider implementing response compression (gzip/brotli)".to_string());
            recommendations.push("Implement pagination for large data sets".to_string());
        }

        if let Some(bandwidth) = metrics.network_conditions.estimated_bandwidth_mbps {
            if bandwidth < 1.0 {
                recommendations.push("Network bandwidth appears limited - consider optimizing payload size".to_string());
            }
        }

        recommendations.push("Monitor network conditions and server response times".to_string());
        recommendations.push("Implement retry logic with exponential backoff".to_string());

        recommendations
    }

    fn determine_severity(&self, total_time_ms: u64) -> PerformanceSeverity {
        match total_time_ms {
            0..=100 => PerformanceSeverity::Excellent,
            101..=500 => PerformanceSeverity::Good,
            501..=1000 => PerformanceSeverity::Average,
            1001..=3000 => PerformanceSeverity::Poor,
            _ => PerformanceSeverity::Critical,
        }
    }

    pub async fn run_performance_test(&self, url: &str, iterations: u32) -> Result<Vec<PerformanceAnalysis>> {
        let mut results = Vec::new();

        info!("Running performance test with {} iterations for: {}", iterations, url);

        for i in 1..=iterations {
            info!("Iteration {}/{}", i, iterations);

            let request = HttpRequestBuilder {
                method: "GET".to_string(),
                url: url.to_string(),
                headers: HashMap::new(),
                body: None,
                timeout_seconds: 30,
                follow_redirects: true,
                verify_ssl: true,
            };

            match self.analyze_request(&request).await {
                Ok(analysis) => {
                    info!("Iteration {} completed: {}ms", i, analysis.metrics.total_time_ms);
                    results.push(analysis);
                }
                Err(e) => {
                    warn!("Iteration {} failed: {}", i, e);
                }
            }

            // Small delay between requests
            if i < iterations {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Ok(results)
    }

    pub fn generate_summary_report(&self, analyses: &[PerformanceAnalysis]) -> String {
        if analyses.is_empty() {
            return "No performance data available".to_string();
        }

        let total_requests = analyses.len();
        let response_times: Vec<u64> = analyses.iter()
            .map(|a| a.metrics.total_time_ms)
            .collect();

        let avg_time = response_times.iter().sum::<u64>() / total_requests as u64;
        let min_time = response_times.iter().min().unwrap_or(&0);
        let max_time = response_times.iter().max().unwrap_or(&0);

        let mut report = String::new();
        report.push_str("=== PERFORMANCE ANALYSIS SUMMARY ===\n\n");
        report.push_str(&format!("Total Requests: {}\n", total_requests));
        report.push_str(&format!("Average Response Time: {}ms\n", avg_time));
        report.push_str(&format!("Minimum Response Time: {}ms\n", min_time));
        report.push_str(&format!("Maximum Response Time: {}ms\n", max_time));

        // Performance distribution
        let excellent = analyses.iter().filter(|a| matches!(a.severity, PerformanceSeverity::Excellent)).count();
        let good = analyses.iter().filter(|a| matches!(a.severity, PerformanceSeverity::Good)).count();
        let average = analyses.iter().filter(|a| matches!(a.severity, PerformanceSeverity::Average)).count();
        let poor = analyses.iter().filter(|a| matches!(a.severity, PerformanceSeverity::Poor)).count();
        let critical = analyses.iter().filter(|a| matches!(a.severity, PerformanceSeverity::Critical)).count();

        report.push_str("\nPerformance Distribution:\n");
        report.push_str(&format!("• Excellent (<100ms): {}\n", excellent));
        report.push_str(&format!("• Good (100-500ms): {}\n", good));
        report.push_str(&format!("• Average (500-1000ms): {}\n", average));
        report.push_str(&format!("• Poor (1000-3000ms): {}\n", poor));
        report.push_str(&format!("• Critical (>3000ms): {}\n", critical));

        if max_time > &6000 {
            report.push_str("\n⚠️  CRITICAL PERFORMANCE ISSUES DETECTED!\n");
            report.push_str("Some requests exceeded 6 seconds response time.\n");
        }

        report.push_str("\nKey Factors Contributing to 6000ms Response Times:\n");
        report.push_str("1. Network Latency - High RTT to target server\n");
        report.push_str("2. Server Processing Time - Slow backend processing\n");
        report.push_str("3. DNS Resolution Delays - Slow DNS lookups\n");
        report.push_str("4. Connection Establishment - TCP/TLS handshake overhead\n");
        report.push_str("5. Network Congestion - Bandwidth limitations\n");
        report.push_str("6. Server Load - High server resource utilization\n");
        report.push_str("7. Geographic Distance - Physical distance to server\n");
        report.push_str("8. ISP Throttling - Provider-level bandwidth limits\n");

        report
    }
}
