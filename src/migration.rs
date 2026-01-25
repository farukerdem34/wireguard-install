use crate::interface::save_multi_interface_config;
use crate::models::{GlobalSettings, InterfaceConfig, MultiInterfaceConfig};
use std::collections::HashMap;
use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;

const OLD_PARAMS_PATH: &str = "/etc/wireguard/params";
const BACKUP_PARAMS_PATH: &str = "/etc/wireguard/params.backup";

pub struct OldWireguardParams {
    pub server_pub_ip: Ipv4Addr,
    pub server_pub_nic: String,
    pub server_wg_nic: String,
    pub server_wg_ipv4: Ipv4Addr,
    pub server_port: u16,
    pub server_priv_key: String,
    pub server_pub_key: String,
    pub client_dns_1: Ipv4Addr,
    pub client_dns_2: Ipv4Addr,
    pub allowed_ips: String,
}

pub fn needs_migration() -> bool {
    Path::new(OLD_PARAMS_PATH).exists() && !Path::new("/etc/wireguard/interfaces.json").exists()
}

pub fn migrate_existing_installation() -> Result<(), String> {
    if !needs_migration() {
        return Ok(());
    }

    println!("🔄 Migrating existing WireGuard installation to multi-interface format...");

    // Read old parameters
    let old_params = read_old_params()?;

    // Create new multi-interface configuration
    let mut config = MultiInterfaceConfig::new();

    // Set global settings from old params
    config.global_settings = GlobalSettings {
        server_pub_ip: old_params.server_pub_ip,
        server_pub_nic: old_params.server_pub_nic.clone(),
        dns_servers: vec![old_params.client_dns_1, old_params.client_dns_2],
        allowed_ips: old_params.allowed_ips.clone(),
    };

    // Determine subnet from server IP
    let subnet = determine_subnet_from_ip(&old_params.server_wg_ipv4);

    // Create interface configuration from old wg0
    let wg0_config = InterfaceConfig {
        name: old_params.server_wg_nic.clone(),
        subnet: subnet.clone(),
        server_ip: old_params.server_wg_ipv4,
        port: old_params.server_port,
        private_key: old_params.server_priv_key.clone(),
        public_key: old_params.server_pub_key.clone(),
        created_at: "migrated".to_string(),
        active: true,
    };

    // Add the migrated interface
    config
        .interfaces
        .insert(old_params.server_wg_nic.clone(), wg0_config);
    config.next_suggested_port = old_params.server_port + 1;

    // Save new configuration
    save_multi_interface_config(&config)?;

    // Backup old params file
    if let Err(e) = fs::rename(OLD_PARAMS_PATH, BACKUP_PARAMS_PATH) {
        println!("Warning: Failed to backup old params file: {}", e);
    }

    println!("✅ Migration completed successfully!");
    println!(
        "   • Interface '{}' migrated with subnet {}",
        old_params.server_wg_nic, subnet
    );
    println!("   • Old configuration backed up to {}", BACKUP_PARAMS_PATH);

    Ok(())
}

fn read_old_params() -> Result<OldWireguardParams, String> {
    let content = fs::read_to_string(OLD_PARAMS_PATH)
        .map_err(|e| format!("Failed to read old params file: {}", e))?;

    let mut params = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            params.insert(key.to_string(), value.to_string());
        }
    }

    // Parse required parameters
    let server_pub_ip = parse_ip(&params, "SERVER_PUB_IP")?;
    let server_pub_nic = get_param(&params, "SERVER_PUB_NIC")?;
    let server_wg_nic = get_param(&params, "SERVER_WG_NIC")?;
    let server_wg_ipv4 = parse_ip(&params, "SERVER_WG_IPV4")?;
    let server_port = parse_port(&params, "SERVER_PORT")?;
    let server_priv_key = get_param(&params, "SERVER_PRIV_KEY")?;
    let server_pub_key = get_param(&params, "SERVER_PUB_KEY")?;
    let client_dns_1 = parse_ip(&params, "CLIENT_DNS_1")?;
    let client_dns_2 = parse_ip(&params, "CLIENT_DNS_2")?;
    let allowed_ips = params
        .get("ALLOWED_IPS")
        .unwrap_or(&"0.0.0.0/0".to_string())
        .clone();

    Ok(OldWireguardParams {
        server_pub_ip,
        server_pub_nic,
        server_wg_nic,
        server_wg_ipv4,
        server_port,
        server_priv_key,
        server_pub_key,
        client_dns_1,
        client_dns_2,
        allowed_ips,
    })
}

fn get_param(params: &HashMap<String, String>, key: &str) -> Result<String, String> {
    params
        .get(key)
        .ok_or_else(|| format!("Missing required parameter: {}", key))
        .map(|s| s.clone())
}

fn parse_ip(params: &HashMap<String, String>, key: &str) -> Result<Ipv4Addr, String> {
    let value = get_param(params, key)?;
    value
        .parse::<Ipv4Addr>()
        .map_err(|_| format!("Invalid IP address for {}: {}", key, value))
}

fn parse_port(params: &HashMap<String, String>, key: &str) -> Result<u16, String> {
    let value = get_param(params, key)?;
    value
        .parse::<u16>()
        .map_err(|_| format!("Invalid port number for {}: {}", key, value))
}

fn determine_subnet_from_ip(server_ip: &Ipv4Addr) -> String {
    let octets = server_ip.octets();

    // Determine subnet based on the IP address
    // Most common case: assume /24 subnet
    format!("{}.{}.{}.0/24", octets[0], octets[1], octets[2])
}

pub fn check_migration_status() -> Result<(), String> {
    if needs_migration() {
        println!("📌 Old WireGuard installation detected.");
        println!(
            "   This installation will be automatically migrated to support multiple interfaces."
        );
        println!("   Your existing configuration and clients will be preserved.");
        println!();

        migrate_existing_installation()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_determine_subnet_from_ip() {
        let ip = "10.19.11.1".parse::<Ipv4Addr>().unwrap();
        assert_eq!(determine_subnet_from_ip(&ip), "10.19.11.0/24");

        let ip = "172.16.50.1".parse::<Ipv4Addr>().unwrap();
        assert_eq!(determine_subnet_from_ip(&ip), "172.16.50.0/24");

        let ip = "192.168.100.1".parse::<Ipv4Addr>().unwrap();
        assert_eq!(determine_subnet_from_ip(&ip), "192.168.100.0/24");
    }
}
