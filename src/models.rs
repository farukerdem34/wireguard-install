use std::net::Ipv4Addr;

pub struct InstallAnswers {
    pub server_pub_ip: Ipv4Addr,
    pub server_public_nic: String,
    pub server_pub_ipv6: Option<String>,
    pub server_wg_nic: String,
    pub server_wg_ip: Ipv4Addr,
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
