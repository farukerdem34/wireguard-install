use crate::client;

/// Example usage of the client module
/// This shows how to create a new WireGuard client
pub fn example_client_creation() {
    println!("Example: Creating a new WireGuard client");

    match client::new_client() {
        Ok(()) => println!("✅ Client created successfully!"),
        Err(e) => println!("❌ Error creating client: {}", e),
    }
}

/// Example of loading WireGuard parameters
pub fn example_load_params() {
    println!("Example: Loading WireGuard server parameters");

    match client::load_wireguard_params() {
        Ok(params) => {
            println!("✅ Parameters loaded successfully:");
            println!("  - Server WG NIC: {}", params.server_wg_nic);
            println!("  - Server IPv4: {}", params.server_wg_ipv4);
            println!("  - Server Port: {}", params.server_port);
            println!(
                "  - DNS Servers: {}, {}",
                params.client_dns_1, params.client_dns_2
            );
        }
        Err(e) => println!("❌ Error loading parameters: {}", e),
    }
}
