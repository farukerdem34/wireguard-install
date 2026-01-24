use std::net::Ipv4Addr;

pub struct InstallAnswers {
    pub server_pub_ip: Ipv4Addr,
    pub server_public_nic: String,
    pub server_pub_ipv6: Option<String>,
    pub server_wg_nic: String,
    pub server_wg_ip: Ipv4Addr,
    pub server_wg_port: u16,
    pub client_dns_1: Ipv4Addr,
    pub client_dns_2: Ipv4Addr,
    pub allowed_ips: String,
}
