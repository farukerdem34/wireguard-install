use std::env;
use wireguard_install::interface::{
    create_new_interface, list_interfaces, load_multi_interface_config,
};
use wireguard_install::migration::needs_migration;
use wireguard_install::models::MultiInterfaceConfig;
use wireguard_install::network::{
    detect_subnet_conflicts, get_server_ip_from_subnet, suggest_available_port, validate_subnet,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("WireGuard Multi-Interface Test Tool");
        println!("Usage:");
        println!("  {} test-validation", args[0]);
        println!("  {} test-interfaces", args[0]);
        println!("  {} test-migration", args[0]);
        return Ok(());
    }

    match args[1].as_str() {
        "test-validation" => test_network_validation(),
        "test-interfaces" => test_interface_management(),
        "test-migration" => test_migration_detection(),
        _ => {
            println!("Unknown command: {}", args[1]);
            Ok(())
        }
    }
}

fn test_network_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing network validation...");

    // Test valid subnets
    let valid_subnets = [
        "10.19.0.0/16",
        "172.16.50.0/24",
        "192.168.100.0/28",
        "10.0.0.0/8",
    ];

    for subnet in &valid_subnets {
        match validate_subnet(subnet) {
            Ok(network) => println!("✅ {} -> Valid (size: {})", subnet, network.size()),
            Err(e) => println!("❌ {} -> Error: {}", subnet, e),
        }
    }

    // Test invalid subnets
    let invalid_subnets = [
        "8.8.8.0/24",    // Public IP
        "10.0.0.0/31",   // Too small
        "invalid",       // Invalid format
        "192.168.1.0/7", // Too large
    ];

    println!("\nTesting invalid subnets:");
    for subnet in &invalid_subnets {
        match validate_subnet(subnet) {
            Ok(_) => println!("❌ {} -> Should have failed!", subnet),
            Err(e) => println!("✅ {} -> Correctly rejected: {}", subnet, e),
        }
    }

    // Test server IP extraction
    println!("\nTesting server IP extraction:");
    for subnet in &valid_subnets {
        match get_server_ip_from_subnet(subnet) {
            Ok(ip) => println!("✅ {} -> Server IP: {}", subnet, ip),
            Err(e) => println!("❌ {} -> Error: {}", subnet, e),
        }
    }

    Ok(())
}

fn test_interface_management() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing interface management...");

    // Test configuration loading
    match load_multi_interface_config() {
        Ok(config) => {
            println!("✅ Configuration loaded successfully");
            println!("   Interfaces: {}", config.interfaces.len());
            println!("   Next suggested port: {}", config.next_suggested_port);

            // Test conflict detection
            println!("\nTesting subnet conflict detection:");
            let test_subnets = ["10.19.11.0/24", "172.16.0.0/24", "192.168.1.0/24"];

            for subnet in &test_subnets {
                match detect_subnet_conflicts(subnet, &config) {
                    Ok(_) => println!("✅ {} -> No conflicts", subnet),
                    Err(e) => println!("⚠️  {} -> Conflict: {}", subnet, e),
                }
            }

            // Test port suggestions
            let used_ports: Vec<u16> = config.interfaces.values().map(|i| i.port).collect();
            let suggested_port = suggest_available_port(&used_ports, 51820);
            println!("\nPort suggestion test:");
            println!("✅ Used ports: {:?}", used_ports);
            println!("✅ Suggested port: {}", suggested_port);
        }
        Err(e) => {
            println!("ℹ️  No existing configuration: {}", e);
            println!("✅ This is expected for new installations");
        }
    }

    Ok(())
}

fn test_migration_detection() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing migration detection...");

    if needs_migration() {
        println!("✅ Old WireGuard installation detected - migration needed");
    } else {
        println!("ℹ️  No migration needed (no old installation found)");
    }

    Ok(())
}
