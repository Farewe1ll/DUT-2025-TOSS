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
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct NetworkPacket {
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    pub dst_port: u16,
    #[allow(dead_code)]
    pub protocol: String,
    pub payload: Vec<u8>,
    #[allow(dead_code)]
    pub timestamp: chrono::DateTime<chrono::Utc>,
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

pub struct PacketCapture {
    interface: String,
    filter: String,
    packet_sender: mpsc::UnboundedSender<NetworkPacket>,
}

impl PacketCapture {
    pub fn new(
        interface: String,
        filter: String,
        packet_sender: mpsc::UnboundedSender<NetworkPacket>,
    ) -> Self {
        Self {
            interface,
            filter,
            packet_sender,
        }
    }    pub async fn start_capture(&self) -> Result<()> {
        let device = Device::list()?
            .into_iter()
            .find(|d| d.name == self.interface)
            .ok_or_else(|| anyhow!("Interface {} not found", self.interface))?;

        info!("Starting packet capture on interface: {}", self.interface);

        // Create capture with better error handling
        let mut cap = match Capture::from_device(device) {
            Ok(cap) => cap
                .promisc(true)
                .snaplen(65535)
                .buffer_size(1_000_000)
                .timeout(1000)
                .open()?,
            Err(e) => {
                error!("Failed to create capture: {}", e);
                return Err(e.into());
            }
        };

        // Apply filter with error handling
        if let Err(e) = cap.filter(&self.filter, true) {
            error!("Failed to apply filter '{}': {}", self.filter, e);
            return Err(e.into());
        }

        let sender = self.packet_sender.clone();

        // Spawn blocking task for packet capture
        tokio::task::spawn_blocking(move || {
            info!("Packet capture loop started");
            let mut packet_count = 0;

            loop {
                match cap.next_packet() {
                    Ok(packet) => {
                        packet_count += 1;
                        if packet_count % 100 == 0 {
                            info!("Captured {} packets", packet_count);
                        }

                        if let Some(network_packet) = Self::parse_packet(packet.data) {
                            if let Err(e) = sender.send(network_packet) {
                                error!("Failed to send packet: {}", e);
                                break;
                            }
                        }
                    }
                    Err(pcap::Error::TimeoutExpired) => {
                        // Timeout is normal, continue
                        continue;
                    }
                    Err(e) => {
                        error!("Error capturing packet: {}", e);
                        break;
                    }
                }
            }

            info!("Packet capture loop ended");
        });

        Ok(())
    }

    fn parse_packet(data: &[u8]) -> Option<NetworkPacket> {
        let ethernet = EthernetPacket::new(data)?;

        match ethernet.get_ethertype() {
            EtherTypes::Ipv4 => {
                let ipv4 = Ipv4Packet::new(ethernet.payload())?;

                match ipv4.get_next_level_protocol() {
                    IpNextHeaderProtocols::Tcp => {
                        let tcp = TcpPacket::new(ipv4.payload())?;

                        Some(NetworkPacket {
                            src_ip: ipv4.get_source().to_string(),
                            dst_ip: ipv4.get_destination().to_string(),
                            src_port: tcp.get_source(),
                            dst_port: tcp.get_destination(),
                            protocol: "TCP".to_string(),
                            payload: tcp.payload().to_vec(),
                            timestamp: chrono::Utc::now(),
                        })
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

pub struct HttpParser;

impl HttpParser {    pub fn parse_http_request(packet: &NetworkPacket) -> Option<HttpRequest> {
        if packet.payload.is_empty() || packet.payload.len() < 10 {
            return None;
        }

        // Only process packets on standard HTTP ports
        if packet.dst_port != 80 && packet.dst_port != 443 && packet.src_port != 80 && packet.src_port != 443 {
            return None;
        }

        let payload_str = String::from_utf8_lossy(&packet.payload);

        // Check if this looks like an HTTP request
        if !payload_str.starts_with("GET ") && !payload_str.starts_with("POST ") &&
           !payload_str.starts_with("PUT ") && !payload_str.starts_with("DELETE ") &&
           !payload_str.starts_with("HEAD ") && !payload_str.starts_with("OPTIONS ") &&
           !payload_str.starts_with("PATCH ") {
            return None;
        }

        let lines: Vec<&str> = payload_str.lines().collect();

        if lines.is_empty() {
            return None;
        }

        // Parse request line
        let request_line_parts: Vec<&str> = lines[0].split_whitespace().collect();
        if request_line_parts.len() < 3 {
            return None;
        }

        let method = request_line_parts[0].to_string();
        let path = request_line_parts[1].to_string();

        // Parse headers
        let mut headers = HashMap::new();
        let mut header_end = 1;

        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim().is_empty() {
                header_end = i + 1;
                break;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        // Extract host for full URL
        let host = headers.get("host").cloned().unwrap_or_else(|| {
            format!("{}:{}", packet.dst_ip, packet.dst_port)
        });

        let scheme = if packet.dst_port == 443 { "https" } else { "http" };
        let url = format!("{}://{}{}", scheme, host, path);

        // Parse body
        let body = if header_end < lines.len() {
            lines[header_end..].join("\n").into_bytes()
        } else {
            Vec::new()
        };

        Some(HttpRequest {
            method,
            url,
            headers,
            body,
            source_ip: packet.src_ip.clone(),
            source_port: packet.src_port,
        })
    }
}
