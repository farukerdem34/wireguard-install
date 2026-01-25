#[cfg(test)]
mod tests {
    use crate::models::MultiInterfaceConfig;
    use crate::network::{get_server_ip_from_subnet, suggest_available_port, validate_subnet};

    #[test]
    fn test_subnet_validation() {
        // Test valid subnets
        assert!(validate_subnet("10.19.0.0/16").is_ok());
        assert!(validate_subnet("172.16.50.0/24").is_ok());
        assert!(validate_subnet("192.168.100.0/28").is_ok());

        // Test invalid subnets
        assert!(validate_subnet("8.8.8.0/24").is_err()); // Public IP
        assert!(validate_subnet("10.0.0.0/31").is_err()); // Too small
        assert!(validate_subnet("invalid").is_err()); // Invalid format
    }

    #[test]
    fn test_server_ip_extraction() {
        let subnet = "10.19.11.0/24";
        let server_ip = get_server_ip_from_subnet(subnet).unwrap();
        assert_eq!(server_ip.to_string(), "10.19.11.1");

        let subnet = "172.16.50.0/28";
        let server_ip = get_server_ip_from_subnet(subnet).unwrap();
        assert_eq!(server_ip.to_string(), "172.16.50.1");
    }

    #[test]
    fn test_port_suggestion() {
        let used_ports = vec![51820, 51821, 51823];
        let suggested = suggest_available_port(&used_ports, 51820);
        assert_eq!(suggested, 51822); // Should suggest next available

        let no_used_ports = vec![];
        let suggested = suggest_available_port(&no_used_ports, 51820);
        assert_eq!(suggested, 51820); // Should suggest starting port
    }

    #[test]
    fn test_multi_interface_config_creation() {
        let config = MultiInterfaceConfig::new();
        assert_eq!(config.interfaces.len(), 0);
        assert_eq!(config.next_suggested_port, 51820);
        assert_eq!(config.global_settings.dns_servers.len(), 2);
    }

    #[test]
    fn test_interface_name_suggestion() {
        let config = MultiInterfaceConfig::new();
        let suggested = config.get_next_interface_name();
        assert_eq!(suggested, "wg0");

        // Test with existing interfaces
        let mut config_with_interfaces = MultiInterfaceConfig::new();
        config_with_interfaces.interfaces.insert(
            "wg0".to_string(),
            crate::models::InterfaceConfig {
                name: "wg0".to_string(),
                subnet: "10.19.11.0/24".to_string(),
                server_ip: "10.19.11.1".parse().unwrap(),
                port: 51820,
                private_key: "test_key".to_string(),
                public_key: "test_pub_key".to_string(),
                created_at: "2026-01-25T10:00:00Z".to_string(),
                active: true,
            },
        );

        let suggested = config_with_interfaces.get_next_interface_name();
        assert_eq!(suggested, "wg1");
    }
}
