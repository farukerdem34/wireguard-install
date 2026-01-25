use crate::models::{InterfaceConfig, MultiInterfaceConfig};
use ipnetwork::{IpNetwork, Ipv4Network};
use std::net::Ipv4Addr;
use std::process::Command;

pub fn validate_subnet(subnet_str: &str) -> Result<Ipv4Network, String> {
    let network = subnet_str
        .parse::<IpNetwork>()
        .map_err(|_| "Invalid CIDR format. Please use format like 10.19.0.0/24")?;

    match network {
        IpNetwork::V4(ipv4_net) => {
            // Ensure it's a private network range
            if !is_private_network(&ipv4_net) {
                return Err("Subnet must be within private network ranges (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)".to_string());
            }

            // Ensure minimum subnet size (/30 = 4 addresses)
            if ipv4_net.prefix() > 30 {
                return Err("Subnet too small. Minimum size is /30 (4 addresses)".to_string());
            }

            // Ensure maximum subnet size (/8 = 16M addresses)
            if ipv4_net.prefix() < 8 {
                return Err("Subnet too large. Maximum size is /8".to_string());
            }

            Ok(ipv4_net)
        }
        IpNetwork::V6(_) => Err("IPv6 subnets are not supported yet".to_string()),
    }
}

pub fn is_private_network(network: &Ipv4Network) -> bool {
    let network_addr = network.network();

    // 10.0.0.0/8
    if network_addr.octets()[0] == 10 {
        return true;
    }

    // 172.16.0.0/12
    if network_addr.octets()[0] == 172 && (16..=31).contains(&network_addr.octets()[1]) {
        return true;
    }

    // 192.168.0.0/16
    if network_addr.octets()[0] == 192 && network_addr.octets()[1] == 168 {
        return true;
    }

    false
}

pub fn detect_subnet_conflicts(
    new_subnet: &str,
    config: &MultiInterfaceConfig,
) -> Result<(), String> {
    let new_network = validate_subnet(new_subnet)?;

    for (interface_name, interface_config) in &config.interfaces {
        let existing_network = validate_subnet(&interface_config.subnet).map_err(|_| {
            format!(
                "Invalid existing subnet in interface {}: {}",
                interface_name, interface_config.subnet
            )
        })?;

        if new_network.overlaps(existing_network) {
            return Err(format!(
                "Subnet {} conflicts with existing interface {} ({})",
                new_subnet, interface_name, interface_config.subnet
            ));
        }
    }

    // Check against system routes
    if let Err(e) = check_system_route_conflicts(&new_network) {
        return Err(format!("System route conflict: {}", e));
    }

    Ok(())
}

pub fn check_system_route_conflicts(network: &Ipv4Network) -> Result<(), String> {
    // Check system routes using 'ip route' command
    let output = Command::new("ip")
        .args(&["route", "show"])
        .output()
        .map_err(|e| format!("Failed to check system routes: {}", e))?;

    if !output.status.success() {
        return Err("Failed to execute 'ip route show'".to_string());
    }

    let routes_output = String::from_utf8_lossy(&output.stdout);

    for line in routes_output.lines() {
        if let Some(route_network) = extract_network_from_route(line) {
            if network.overlaps(route_network) {
                return Err(format!(
                    "Subnet {} conflicts with system route: {}",
                    network,
                    line.trim()
                ));
            }
        }
    }

    Ok(())
}

fn extract_network_from_route(route_line: &str) -> Option<Ipv4Network> {
    let parts: Vec<&str> = route_line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let network_str = parts[0];

    // Handle different route formats
    if network_str == "default" {
        return None; // Skip default routes
    }

    // Try to parse as CIDR
    if let Ok(network) = network_str.parse::<IpNetwork>() {
        if let IpNetwork::V4(ipv4_net) = network {
            return Some(ipv4_net);
        }
    }

    // Try to parse as single IP (add /32)
    if let Ok(ip) = network_str.parse::<Ipv4Addr>() {
        if let Ok(network) = format!("{}/32", ip).parse::<Ipv4Network>() {
            return Some(network);
        }
    }

    None
}

pub fn is_port_in_use(port: u16) -> bool {
    // Check if UDP port is bound using ss command
    let output = Command::new("ss").args(&["-ulpn"]).output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        return stdout
            .lines()
            .any(|line| line.contains(&format!(":{}", port)) && line.contains("udp"));
    }

    // Fallback to netstat if ss is not available
    let output = Command::new("netstat").args(&["-ulpn"]).output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        return stdout
            .lines()
            .any(|line| line.contains(&format!(":{}", port)) && line.contains("udp"));
    }

    false
}

pub fn suggest_available_port(existing_ports: &[u16], start_port: u16) -> u16 {
    let mut port = start_port;

    loop {
        if !existing_ports.contains(&port) && !is_port_in_use(port) {
            return port;
        }
        port += 1;

        // Safety check to prevent infinite loop
        if port > 65535 {
            port = 1024; // Start from well-known port range end
        }
        if port == start_port {
            break; // Full circle, no available ports found
        }
    }

    start_port // Return original if no available port found
}

pub fn get_server_ip_from_subnet(subnet: &str) -> Result<Ipv4Addr, String> {
    let network = validate_subnet(subnet)?;

    // Use the first usable IP in the subnet as server IP
    let mut hosts = network.iter();

    // Skip network address
    hosts.next();

    // Return the first host IP (which will be the server IP)
    if let Some(first_host) = hosts.next() {
        Ok(first_host)
    } else {
        Err("Subnet too small to assign server IP".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_subnet() {
        assert!(validate_subnet("10.19.0.0/24").is_ok());
        assert!(validate_subnet("172.16.50.0/28").is_ok());
        assert!(validate_subnet("192.168.100.0/16").is_ok());

        assert!(validate_subnet("8.8.8.0/24").is_err()); // Public IP
        assert!(validate_subnet("10.0.0.0/31").is_err()); // Too small
        assert!(validate_subnet("invalid").is_err()); // Invalid format
    }

    #[test]
    fn test_get_server_ip_from_subnet() {
        assert_eq!(
            get_server_ip_from_subnet("10.19.11.0/24").unwrap(),
            "10.19.11.1".parse::<Ipv4Addr>().unwrap()
        );

        assert_eq!(
            get_server_ip_from_subnet("172.16.50.0/28").unwrap(),
            "172.16.50.1".parse::<Ipv4Addr>().unwrap()
        );
    }

    #[test]
    fn test_is_private_network() {
        assert!(is_private_network(&"10.0.0.0/24".parse().unwrap()));
        assert!(is_private_network(&"172.16.0.0/24".parse().unwrap()));
        assert!(is_private_network(&"192.168.1.0/24".parse().unwrap()));
        assert!(!is_private_network(&"8.8.8.0/24".parse().unwrap()));
    }
}
