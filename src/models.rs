use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::Ipv4Addr;

pub struct InstallAnswers {
    pub server_pub_ip: Ipv4Addr,
    pub server_public_nic: String,
    pub server_pub_ipv6: Option<String>,
    pub server_wg_nic: String,
    pub server_wg_ip: Ipv4Addr,
    pub server_wg_subnet: String, // Add subnet field
    pub server_wg_port: u16,
    pub server_priv_key: String,
    pub server_pub_key: String,
    pub client_dns_1: Ipv4Addr,
    pub client_dns_2: Ipv4Addr,
    pub allowed_ips: String,
}

pub struct VersionInfo {
    pub major_version: u32,
    pub minor_version: Option<u32>,
    pub full_version: String,
}

impl VersionInfo {
    pub fn new(version_string: &str) -> Self {
        let parts: Vec<&str> = version_string.split('.').collect();
        let major_version = parts
            .get(0)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let minor_version = parts.get(1).and_then(|v| v.parse::<u32>().ok());

        VersionInfo {
            major_version,
            minor_version,
            full_version: version_string.to_string(),
        }
    }
}

// Multi-interface support structures
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MultiInterfaceConfig {
    pub interfaces: HashMap<String, InterfaceConfig>,
    pub global_settings: GlobalSettings,
    pub next_suggested_port: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InterfaceConfig {
    pub name: String,
    pub subnet: String,      // Store as CIDR string (e.g., "10.19.23.0/24")
    pub server_ip: Ipv4Addr, // Server's IP within the subnet
    pub port: u16,
    pub private_key: String,
    pub public_key: String,
    pub created_at: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GlobalSettings {
    pub server_pub_ip: Ipv4Addr,
    pub server_pub_nic: String,
    pub dns_servers: Vec<Ipv4Addr>,
    pub allowed_ips: String,
}

impl MultiInterfaceConfig {
    pub fn new() -> Self {
        Self {
            interfaces: HashMap::new(),
            global_settings: GlobalSettings {
                server_pub_ip: "0.0.0.0".parse().unwrap(),
                server_pub_nic: String::new(),
                dns_servers: vec!["1.1.1.1".parse().unwrap(), "1.0.0.1".parse().unwrap()],
                allowed_ips: "0.0.0.0/0".to_string(),
            },
            next_suggested_port: 51820,
        }
    }

    pub fn get_next_interface_name(&self) -> String {
        let mut counter = 0;
        loop {
            let suggested = format!("wg{}", counter);
            if !self.interfaces.contains_key(&suggested) {
                return suggested;
            }
            counter += 1;
        }
    }

    pub fn get_next_suggested_port(&self) -> u16 {
        let mut port = self.next_suggested_port;
        let used_ports: Vec<u16> = self.interfaces.values().map(|i| i.port).collect();

        while used_ports.contains(&port) {
            port += 1;
        }
        port
    }
}
