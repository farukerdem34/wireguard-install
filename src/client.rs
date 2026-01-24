use crate::utils::{clear_terminal, wait_for_key_press_with_message};
use dialoguer::{Confirm, Input, Select};
use qrcode::render::unicode;
use qrcode::QrCode;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::Command;

/// WireGuard server configuration loaded from /etc/wireguard/params
#[derive(Debug, Clone)]
pub struct WireguardParams {
    pub server_pub_nic: String,
    pub server_wg_nic: String,
    pub server_wg_ipv4: Ipv4Addr,
    pub server_wg_ipv6: Option<String>,
    pub server_port: u16,
    pub server_priv_key: String,
    pub server_pub_key: String,
    pub client_dns_1: Ipv4Addr,
    pub client_dns_2: Ipv4Addr,
    pub allowed_ips: String,
}

/// Client configuration structure
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub name: String,
    pub private_key: String,
    pub public_key: String,
    pub preshared_key: String,
    pub ipv4: Ipv4Addr,
    pub ipv6: Option<String>,
    pub home_dir: PathBuf,
    pub use_dns: bool,
    pub allowed_ips: String,
}

/// Validate IPv4 address is in server subnet and not in use
fn validate_ipv4_address(
    ip: &str,
    server_ipv4: &Ipv4Addr,
    server_wg_nic: &str,
) -> Result<Ipv4Addr, String> {
    // Parse the IP address
    let parsed_ip = ip
        .parse::<Ipv4Addr>()
        .map_err(|_| "Invalid IPv4 address format".to_string())?;

    // Extract base IP (first 3 octets) from server IP
    let server_octets = server_ipv4.octets();
    let server_base = format!(
        "{}.{}.{}",
        server_octets[0], server_octets[1], server_octets[2]
    );

    let parsed_octets = parsed_ip.octets();
    let parsed_base = format!(
        "{}.{}.{}",
        parsed_octets[0], parsed_octets[1], parsed_octets[2]
    );

    // Check if IP is in same subnet
    if server_base != parsed_base {
        return Err(format!("IP address must be in subnet {}.x", server_base));
    }

    // Check if it's the server's IP
    if parsed_ip == *server_ipv4 {
        return Err("Cannot use server's IP address".to_string());
    }

    // Check if IP is already in use
    let config_path = format!("/etc/wireguard/{}.conf", server_wg_nic);
    if let Ok(config_content) = fs::read_to_string(&config_path) {
        let search_pattern = format!("{}/32", parsed_ip);
        if config_content.contains(&search_pattern) {
            return Err(format!("IP address {} is already in use", parsed_ip));
        }
    }

    Ok(parsed_ip)
}

/// Validate IPv6 address is in server subnet and not in use
fn validate_ipv6_address(
    ip: &str,
    server_ipv6: &str,
    server_wg_nic: &str,
) -> Result<String, String> {
    // Basic IPv6 format validation (simplified)
    if !ip.contains("::") {
        return Err("Invalid IPv6 address format (must contain '::')".to_string());
    }

    // Extract base IPv6 (everything before ::)
    let server_base = if let Some((prefix, _)) = server_ipv6.split_once("::") {
        prefix
    } else {
        return Err("Invalid server IPv6 configuration".to_string());
    };

    let parsed_base = if let Some((prefix, _)) = ip.split_once("::") {
        prefix
    } else {
        return Err("Invalid IPv6 address format".to_string());
    };

    // Check if IP is in same subnet
    if server_base != parsed_base {
        return Err(format!("IPv6 address must be in subnet {}::x", server_base));
    }

    // Check if it's the server's IPv6
    if ip == server_ipv6 {
        return Err("Cannot use server's IPv6 address".to_string());
    }

    // Check if IP is already in use
    let config_path = format!("/etc/wireguard/{}.conf", server_wg_nic);
    if let Ok(config_content) = fs::read_to_string(&config_path) {
        let search_pattern = format!("{}/128", ip);
        if config_content.contains(&search_pattern) {
            return Err(format!("IPv6 address {} is already in use", ip));
        }
    }

    Ok(ip.to_string())
}

/// Prompt user for client IPv4 address with suggestion
fn prompt_for_client_ipv4(server_ipv4: &Ipv4Addr, server_wg_nic: &str) -> Result<Ipv4Addr, String> {
    // Find suggested IP
    let suggested_ip = find_available_ipv4(server_ipv4, server_wg_nic)?;

    // Extract subnet info for display
    let octets = server_ipv4.octets();
    let subnet = format!("{}.{}.{}", octets[0], octets[1], octets[2]);

    println!("");
    println!("IPv4 Address Configuration");
    println!("");
    println!("Server subnet: {}.x", subnet);
    println!("Suggested next available IP address: {}", suggested_ip);

    loop {
        let choice = Select::new()
            .with_prompt("Choose IPv4 address option")
            .items(&[
                "Use suggested IP address (recommended)",
                "Enter custom IP address",
            ])
            .default(0)
            .interact()
            .map_err(|e| format!("Selection error: {}", e))?;

        match choice {
            0 => {
                // Use suggested IP
                println!("✓ Using IP address: {}", suggested_ip);
                return Ok(suggested_ip);
            }
            1 => {
                // Custom IP input
                let custom_ip: String = Input::new()
                    .with_prompt(&format!(
                        "Enter custom IPv4 address (must be in {}.x)",
                        subnet
                    ))
                    .interact()
                    .map_err(|e| format!("Input error: {}", e))?;

                match validate_ipv4_address(&custom_ip, server_ipv4, server_wg_nic) {
                    Ok(ip) => {
                        println!("✓ Using IP address: {}", ip);
                        return Ok(ip);
                    }
                    Err(e) => {
                        println!("");
                        println!("❌ {}", e);
                        println!("Please try again.");
                        println!("");
                        continue;
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

/// Prompt user for client IPv6 address with suggestion (when IPv6 is available)
/// Returns None if IPv6 is not configured or user chooses to skip
fn prompt_for_client_ipv6(
    server_ipv6: Option<&String>,
    server_wg_nic: &str,
) -> Result<Option<String>, String> {
    // If no server IPv6 configured, return None
    let server_ipv6_str = match server_ipv6 {
        Some(ipv6) => ipv6,
        None => return Ok(None),
    };

    // Find suggested IPv6
    let suggested_ipv6 = find_available_ipv6(Some(server_ipv6_str), server_wg_nic);

    // Extract subnet info for display
    let subnet_prefix = if let Some((prefix, _)) = server_ipv6_str.split_once("::") {
        prefix
    } else {
        "invalid"
    };

    println!("");
    println!("IPv6 Address Configuration");
    println!("");
    println!("Server IPv6 subnet: {}::x", subnet_prefix);

    if let Some(suggested) = &suggested_ipv6 {
        println!("Suggested next available IPv6 address: {}", suggested);

        let choice = Select::new()
            .with_prompt("Choose IPv6 address option")
            .items(&[
                "Use suggested IPv6 address (recommended)",
                "Enter custom IPv6 address",
                "Skip IPv6 configuration",
            ])
            .default(0)
            .interact()
            .map_err(|e| format!("Selection error: {}", e))?;

        match choice {
            0 => {
                // Use suggested IPv6
                println!("✓ Using IPv6 address: {}", suggested);
                Ok(Some(suggested.clone()))
            }
            1 => {
                // Custom IPv6 input
                loop {
                    let custom_ipv6: String = Input::new()
                        .with_prompt(&format!(
                            "Enter custom IPv6 address (must be in {}::x)",
                            subnet_prefix
                        ))
                        .interact()
                        .map_err(|e| format!("Input error: {}", e))?;

                    match validate_ipv6_address(&custom_ipv6, server_ipv6_str, server_wg_nic) {
                        Ok(ip) => {
                            println!("✓ Using IPv6 address: {}", ip);
                            return Ok(Some(ip));
                        }
                        Err(e) => {
                            println!("");
                            println!("❌ {}", e);
                            println!("Please try again.");
                            println!("");
                            // Continue loop
                        }
                    }
                }
            }
            2 => {
                // Skip IPv6
                println!("✓ IPv6 configuration skipped");
                Ok(None)
            }
            _ => unreachable!(),
        }
    } else {
        println!("ℹ️  No IPv6 addresses available in subnet");

        let choice = Select::new()
            .with_prompt("Choose IPv6 option")
            .items(&["Enter custom IPv6 address", "Skip IPv6 configuration"])
            .default(1)
            .interact()
            .map_err(|e| format!("Selection error: {}", e))?;

        match choice {
            0 => {
                // Custom IPv6 input
                loop {
                    let custom_ipv6: String = Input::new()
                        .with_prompt(&format!(
                            "Enter custom IPv6 address (must be in {}::x)",
                            subnet_prefix
                        ))
                        .interact()
                        .map_err(|e| format!("Input error: {}", e))?;

                    match validate_ipv6_address(&custom_ipv6, server_ipv6_str, server_wg_nic) {
                        Ok(ip) => {
                            println!("✓ Using IPv6 address: {}", ip);
                            return Ok(Some(ip));
                        }
                        Err(e) => {
                            println!("");
                            println!("❌ {}", e);
                            println!("Please try again.");
                            println!("");
                            // Continue loop
                        }
                    }
                }
            }
            1 => {
                // Skip IPv6
                println!("✓ IPv6 configuration skipped");
                Ok(None)
            }
            _ => unreachable!(),
        }
    }
}

/// Prompt user whether to include DNS configuration
fn prompt_for_dns_usage() -> Result<bool, String> {
    println!("");
    println!("DNS Configuration");
    println!("");
    println!("Do you want to include DNS settings in the client configuration?");
    println!("This will automatically route DNS queries through the VPN.");

    let use_dns = Confirm::new()
        .with_prompt("Include DNS configuration")
        .default(true)
        .interact()
        .map_err(|e| format!("Selection error: {}", e))?;

    if use_dns {
        println!("✓ DNS configuration will be included");
    } else {
        println!("✓ DNS configuration will be excluded");
    }

    Ok(use_dns)
}

/// Prompt user for allowed IP addresses with validation
fn prompt_for_allowed_ips() -> Result<String, String> {
    println!("");
    println!("Allowed IP Configuration");
    println!("");
    println!("Specify which traffic should be routed through the VPN.");
    println!("Examples:");
    println!("  • 0.0.0.0/0 - Route all traffic through VPN (full tunnel)");
    println!("  • 10.0.0.0/8 - Route only private network traffic");
    println!("  • 192.168.1.0/24 - Route only specific subnet traffic");
    println!("  • Multiple ranges: 10.0.0.0/8,192.168.0.0/16");

    let choice = Select::new()
        .with_prompt("Choose allowed IP configuration")
        .items(&[
            "Route all traffic (0.0.0.0/0) - Recommended for full VPN",
            "Enter custom allowed IPs",
        ])
        .default(0)
        .interact()
        .map_err(|e| format!("Selection error: {}", e))?;

    match choice {
        0 => {
            // Use default all traffic routing
            println!("✓ Using allowed IPs: 0.0.0.0/0");
            Ok("0.0.0.0/0".to_string())
        }
        1 => {
            // Custom allowed IPs input
            loop {
                let custom_ips: String = Input::new()
                    .with_prompt("Enter allowed IPs (comma-separated)")
                    .with_initial_text("0.0.0.0/0")
                    .interact()
                    .map_err(|e| format!("Input error: {}", e))?;

                let custom_ips = custom_ips.trim();

                if custom_ips.is_empty() {
                    println!("");
                    println!("❌ Allowed IPs cannot be empty");
                    println!("Please try again.");
                    println!("");
                    continue;
                }

                // Basic validation: check for valid CIDR-like format
                let is_valid = custom_ips.split(',').all(|ip| {
                    let ip = ip.trim();
                    // Simple regex check for CIDR format (x.x.x.x/y or x.x.x.x)
                    ip.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '/' || c == ':')
                        && (ip.contains('.') || ip.contains(':')) // IPv4 or IPv6
                        && !ip.starts_with('/') && !ip.ends_with('/')
                });

                if !is_valid {
                    println!("");
                    println!(
                        "❌ Invalid IP format. Please use CIDR notation (e.g., 192.168.1.0/24)"
                    );
                    println!("Please try again.");
                    println!("");
                    continue;
                }

                println!("✓ Using allowed IPs: {}", custom_ips);
                return Ok(custom_ips.to_string());
            }
        }
        _ => unreachable!(),
    }
}

/// Main entry point for creating a new WireGuard client
/// This function loads server parameters and guides through interactive client creation
pub fn new_client() -> Result<(), String> {
    println!("Loading WireGuard server configuration...");

    // Step 1: Load server configuration from /etc/wireguard/params
    let params = load_wireguard_params()
        .map_err(|e| format!("Failed to load server configuration: {}", e))?;

    println!("✓ Server configuration loaded successfully");

    // Step 2: Interactive client creation
    println!("");
    println!("Client configuration");
    println!("");
    println!("The client name must consist of alphanumeric character(s). It may also include underscores or dashes and can't exceed 15 chars.");

    let client_name = prompt_and_validate_client_name(&params.server_wg_nic)?;

    // Step 3: Interactive IP address configuration
    let client_ipv4 = prompt_for_client_ipv4(&params.server_wg_ipv4, &params.server_wg_nic)?;
    let client_ipv6 =
        prompt_for_client_ipv6(params.server_wg_ipv6.as_ref(), &params.server_wg_nic)?;

    // Step 4: DNS configuration choice
    let use_dns = prompt_for_dns_usage()?;

    // Step 5: Allowed IPs configuration
    let allowed_ips = prompt_for_allowed_ips()?;

    // Clear terminal after all user inputs are collected and before processing
    clear_terminal();

    println!("🔧 Processing client configuration...");
    println!("✓ Client name: {}", client_name);
    println!("✓ IPv4 address: {}", client_ipv4);
    if let Some(ref ipv6) = client_ipv6 {
        println!("✓ IPv6 address: {}", ipv6);
    }
    println!("✓ DNS enabled: {}", use_dns);
    println!("✓ Allowed IPs: {}", allowed_ips);
    println!();

    // Step 6: Generate client keys
    let (client_private_key, client_public_key, client_preshared_key) = generate_client_keys()?;

    // Step 7: Get home directory for client
    let home_dir = get_home_dir_for_client(&client_name)?;

    // Step 8: Create client configuration
    let client_config = ClientConfig {
        name: client_name.clone(),
        private_key: client_private_key,
        public_key: client_public_key.clone(),
        preshared_key: client_preshared_key.clone(),
        ipv4: client_ipv4,
        ipv6: client_ipv6,
        home_dir,
        use_dns,
        allowed_ips,
    };

    // Step 9: Create configuration file
    create_client_config_file(&client_config, &params)?;

    // Step 10: Add client to server configuration
    add_client_to_server_config(&client_config, &params)?;

    // Step 11: Sync WireGuard configuration
    sync_wireguard_config(&params.server_wg_nic)?;

    // Step 12: Generate QR code and show configuration
    let config_path = client_config.home_dir.join(format!(
        "{}-client-{}.conf",
        params.server_wg_nic, client_config.name
    ));

    // Display configuration information
    println!("");
    println!("✅ Client '{}' created successfully!", client_config.name);
    println!("");
    println!("📁 Configuration file saved to: {}", config_path.display());
    println!("");

    // Show configuration content
    if let Ok(config_content) = fs::read_to_string(&config_path) {
        println!("📋 Client Configuration:");
        println!("─────────────────────────");
        println!("{}", config_content);
        println!("─────────────────────────");
        println!("");
    }

    // Generate and display QR code
    if let Err(e) = generate_qr_code(&config_path) {
        println!("Warning: Failed to generate QR code: {}", e);
    }

    // Wait for user acknowledgment before clearing
    println!("");
    wait_for_key_press_with_message("Press any key to continue and return to the main menu...");

    // Clear terminal after user presses a key
    clear_terminal();

    Ok(())
}

/// Load WireGuard parameters from /etc/wireguard/params file
pub fn load_wireguard_params() -> Result<WireguardParams, String> {
    let params_path = "/etc/wireguard/params";

    // Read the file content
    let content = fs::read_to_string(params_path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                format!("WireGuard parameters file not found at {}\nPlease ensure WireGuard server is properly installed and configured.", params_path)
            },
            std::io::ErrorKind::PermissionDenied => {
                format!("Permission denied reading {}\nPlease run this program with appropriate privileges (sudo).", params_path)
            },
            _ => format!("Failed to read {}: {}", params_path, e),
        })?;

    // Parse key=value pairs
    let vars = parse_shell_vars(&content)?;

    // Load server IPv6 configuration (optional)
    let server_wg_ipv6 = get_optional_var(&vars, "SERVER_WG_IPV6");
    if server_wg_ipv6.is_none() {
        println!("ℹ️  IPv6 not configured in server parameters, using IPv4-only mode");
    }

    // Convert to structured config with validation
    Ok(WireguardParams {
        server_pub_nic: get_required_var(&vars, "SERVER_PUB_NIC")?,
        server_wg_nic: get_required_var(&vars, "SERVER_WG_NIC")?,
        server_wg_ipv4: get_required_var(&vars, "SERVER_WG_IPV4")?
            .parse::<Ipv4Addr>()
            .map_err(|e| format!("Invalid SERVER_WG_IPV4: {}", e))?,
        server_wg_ipv6,
        server_port: get_required_var(&vars, "SERVER_PORT")?
            .parse::<u16>()
            .map_err(|e| format!("Invalid SERVER_PORT: {}", e))?,
        server_priv_key: get_required_var(&vars, "SERVER_PRIV_KEY")?,
        server_pub_key: get_required_var(&vars, "SERVER_PUB_KEY")?,
        client_dns_1: get_required_var(&vars, "CLIENT_DNS_1")?
            .parse::<Ipv4Addr>()
            .map_err(|e| format!("Invalid CLIENT_DNS_1: {}", e))?,
        client_dns_2: get_required_var(&vars, "CLIENT_DNS_2")?
            .parse::<Ipv4Addr>()
            .map_err(|e| format!("Invalid CLIENT_DNS_2: {}", e))?,
        allowed_ips: get_required_var(&vars, "ALLOWED_IPS")?,
    })
}

/// Parse shell variable format (KEY=VALUE and KEY=${VAR})
fn parse_shell_vars(content: &str) -> Result<HashMap<String, String>, String> {
    let mut vars = HashMap::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE format
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            // Handle shell variable substitution like ${VAR} -> VAR
            let processed_value = if value.starts_with("${") && value.ends_with('}') {
                // Extract variable name from ${VAR} and use it as value
                &value[2..value.len() - 1]
            } else {
                value
            };

            vars.insert(key.to_string(), processed_value.to_string());
        } else {
            return Err(format!("Invalid format at line {}: {}", line_num + 1, line));
        }
    }

    Ok(vars)
}

/// Get a required variable from the parsed vars map
fn get_required_var(vars: &HashMap<String, String>, key: &str) -> Result<String, String> {
    vars.get(key)
        .ok_or_else(|| format!("Missing required parameter: {}", key))
        .map(|v| v.clone())
}

/// Get an optional variable from the parsed vars map
/// Returns None if missing or empty, Some(value) if present and valid
fn get_optional_var(vars: &HashMap<String, String>, key: &str) -> Option<String> {
    vars.get(key).and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// Format server endpoint, handling IPv6 bracket requirements
fn format_endpoint(server_ip: &str, port: u16) -> String {
    // If SERVER_PUB_IP is IPv6, add brackets if missing
    if server_ip.contains(':') && !server_ip.starts_with('[') {
        format!("[{}]:{}", server_ip, port)
    } else {
        format!("{}:{}", server_ip, port)
    }
}

/// Prompt user for client name and validate it
fn prompt_and_validate_client_name(server_wg_nic: &str) -> Result<String, String> {
    let name_regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();

    loop {
        let client_name: String = Input::new()
            .with_prompt("Client name")
            .interact()
            .map_err(|e| format!("Input error: {}", e))?;

        // Validate name format
        if !name_regex.is_match(&client_name) {
            println!("");
            println!("Invalid characters in client name. Use only alphanumeric characters, underscores, or dashes.");
            println!("");
            continue;
        }

        // Validate name length
        if client_name.len() >= 16 {
            println!("");
            println!("Client name must be less than 16 characters.");
            println!("");
            continue;
        }

        // Check if client already exists
        let config_path = format!("/etc/wireguard/{}.conf", server_wg_nic);
        if let Ok(content) = fs::read_to_string(&config_path) {
            let search_pattern = format!("### Client {}", client_name);
            if content.contains(&search_pattern) {
                println!("");
                println!("A client with the specified name was already created, please choose another name.");
                println!("");
                continue;
            }
        }

        return Ok(client_name);
    }
}

/// Find an available IPv4 address in the server's subnet
fn find_available_ipv4(server_ipv4: &Ipv4Addr, server_wg_nic: &str) -> Result<Ipv4Addr, String> {
    let config_path = format!("/etc/wireguard/{}.conf", server_wg_nic);
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read server config: {}", e))?;

    // Extract base IP (first 3 octets)
    let octets = server_ipv4.octets();
    let base_ip = format!("{}.{}.{}", octets[0], octets[1], octets[2]);

    // Try IPs from 2 to 254
    for dot_ip in 2..255 {
        let test_ip = format!("{}.{}", base_ip, dot_ip);
        let search_pattern = format!("{}/32", test_ip);

        if !config_content.contains(&search_pattern) {
            return test_ip
                .parse::<Ipv4Addr>()
                .map_err(|e| format!("Failed to parse IP address: {}", e));
        }
    }

    Err("The subnet configured supports only 253 clients.".to_string())
}

/// Find an available IPv6 address in the server's subnet
/// Returns None if IPv6 is not configured or invalid, Some(address) if available
fn find_available_ipv6(server_ipv6: Option<&String>, server_wg_nic: &str) -> Option<String> {
    // If no server IPv6 is configured, return None
    let server_ipv6 = server_ipv6?;

    let config_path = format!("/etc/wireguard/{}.conf", server_wg_nic);
    let config_content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(_) => return None, // Fail gracefully if can't read server config
    };

    // Extract base IPv6 (everything before ::)
    let base_ip = if let Some((prefix, _)) = server_ipv6.split_once("::") {
        prefix
    } else {
        return None; // Invalid IPv6 format, treat as not configured
    };

    // Try IPv6 addresses from 2 to 254
    for dot_ip in 2..255 {
        let test_ipv6 = format!("{}::{}", base_ip, dot_ip);
        let search_pattern = format!("{}/128", test_ipv6);

        if !config_content.contains(&search_pattern) {
            return Some(test_ipv6);
        }
    }

    None // No available IPv6 addresses in subnet
}

/// Generate WireGuard key pair and preshared key
fn generate_client_keys() -> Result<(String, String, String), String> {
    // Generate private key
    let private_key_output = Command::new("wg")
        .arg("genkey")
        .output()
        .map_err(|e| format!("Failed to generate private key: {}", e))?;

    if !private_key_output.status.success() {
        return Err("wg genkey command failed".to_string());
    }

    let private_key = String::from_utf8(private_key_output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in private key: {}", e))?
        .trim()
        .to_string();

    // Generate public key from private key
    let mut public_key_cmd = Command::new("wg")
        .arg("pubkey")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start wg pubkey: {}", e))?;

    // Write private key to stdin
    use std::io::Write;
    if let Some(stdin) = public_key_cmd.stdin.take() {
        let mut stdin = stdin;
        stdin
            .write_all(private_key.as_bytes())
            .map_err(|e| format!("Failed to write to wg pubkey stdin: {}", e))?;
    }

    let public_key_result = public_key_cmd
        .wait_with_output()
        .map_err(|e| format!("Failed to get wg pubkey output: {}", e))?;

    if !public_key_result.status.success() {
        return Err("wg pubkey command failed".to_string());
    }

    let public_key = String::from_utf8(public_key_result.stdout)
        .map_err(|e| format!("Invalid UTF-8 in public key: {}", e))?
        .trim()
        .to_string();

    // Generate preshared key
    let preshared_key_output = Command::new("wg")
        .arg("genpsk")
        .output()
        .map_err(|e| format!("Failed to generate preshared key: {}", e))?;

    if !preshared_key_output.status.success() {
        return Err("wg genpsk command failed".to_string());
    }

    let preshared_key = String::from_utf8(preshared_key_output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in preshared key: {}", e))?
        .trim()
        .to_string();

    Ok((private_key, public_key, preshared_key))
}

/// Get home directory for client (equivalent to getHomeDirForClient bash function)
fn get_home_dir_for_client(_client_name: &str) -> Result<PathBuf, String> {
    // For now, we'll use the current user's home directory
    // In a real implementation, this might need to determine the actual user's home
    if let Some(home) = std::env::var_os("HOME") {
        Ok(PathBuf::from(home))
    } else {
        // Fallback to /root for root user or /tmp
        Ok(PathBuf::from("/root"))
    }
}

/// Create client configuration file
fn create_client_config_file(
    client: &ClientConfig,
    params: &WireguardParams,
) -> Result<(), String> {
    // Use server's public IP for endpoint, format properly for IPv6
    let server_pub_ip =
        std::env::var("SERVER_PUB_IP").unwrap_or_else(|_| params.server_wg_ipv4.to_string());
    let endpoint = format_endpoint(&server_pub_ip, params.server_port);

    // Build address line - conditionally include IPv6
    let address_line = match &client.ipv6 {
        Some(ipv6) => format!("{}/32,{}/128", client.ipv4, ipv6),
        None => format!("{}/32", client.ipv4),
    };

    // Build DNS line - conditionally include based on user preference
    let dns_line = if client.use_dns {
        format!("DNS = {},{}\n", params.client_dns_1, params.client_dns_2)
    } else {
        String::new()
    };

    let config_content = format!(
        "[Interface]\n\
         PrivateKey = {}\n\
         Address = {}\n\
         {}\n\
         # Uncomment the next line to set a custom MTU\n\
         # This might impact performance, so use it only if you know what you are doing\n\
         # See https://github.com/nitred/nr-wg-mtu-finder to find your optimal MTU\n\
         # MTU = 1420\n\n\
         [Peer]\n\
         PublicKey = {}\n\
         PresharedKey = {}\n\
         Endpoint = {}\n\
         AllowedIPs = {}",
        client.private_key,
        address_line,
        dns_line,
        params.server_pub_key,
        client.preshared_key,
        endpoint,
        client.allowed_ips
    );

    let config_path = client.home_dir.join(format!(
        "{}-client-{}.conf",
        params.server_wg_nic, client.name
    ));

    fs::write(&config_path, config_content)
        .map_err(|e| format!("Failed to write client config: {}", e))?;

    Ok(())
}

/// Add client as peer to server configuration
fn add_client_to_server_config(
    client: &ClientConfig,
    params: &WireguardParams,
) -> Result<(), String> {
    let server_config_path = format!("/etc/wireguard/{}.conf", params.server_wg_nic);

    // Build AllowedIPs line - conditionally include IPv6
    let allowed_ips = match &client.ipv6 {
        Some(ipv6) => format!("{}/32,{}/128", client.ipv4, ipv6),
        None => format!("{}/32", client.ipv4),
    };

    let client_peer_config = format!(
        "\n### Client {}\n\
         [Peer]\n\
         PublicKey = {}\n\
         PresharedKey = {}\n\
         AllowedIPs = {}",
        client.name, client.public_key, client.preshared_key, allowed_ips
    );

    // Append client configuration to server config
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&server_config_path)
        .and_then(|mut file| {
            use std::io::Write;
            file.write_all(client_peer_config.as_bytes())
        })
        .map_err(|e| format!("Failed to update server config: {}", e))?;

    Ok(())
}

/// Sync WireGuard configuration with running interface
fn sync_wireguard_config(interface: &str) -> Result<(), String> {
    let output = Command::new("wg")
        .arg("syncconf")
        .arg(interface)
        .arg(format!("/dev/stdin"))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            // Generate stripped config for syncconf
            let strip_output = Command::new("wg-quick")
                .arg("strip")
                .arg(interface)
                .output()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                stdin.write_all(&strip_output.stdout)?;
            }

            child.wait()
        })
        .map_err(|e| format!("Failed to sync WireGuard config: {}", e))?;

    if !output.success() {
        return Err("wg syncconf command failed".to_string());
    }

    Ok(())
}

/// Generate and display QR code for client configuration
fn generate_qr_code(config_path: &PathBuf) -> Result<(), String> {
    let config_content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let code =
        QrCode::new(config_content).map_err(|e| format!("Failed to generate QR code: {}", e))?;

    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();

    println!("");
    println!("Here is your client config file as a QR Code:");
    println!("");
    println!("{}", image);
    println!("");

    Ok(())
}

/// Lists all existing WireGuard clients with enhanced formatting
/// Equivalent to the bash listClients() function
///
/// This function:
/// 1. Loads WireGuard parameters from /etc/wireguard/params
/// 2. Reads the server configuration file to find client entries
/// 3. Displays a numbered list of all clients with enhanced formatting
/// 4. Exits with code 1 if no clients are found (matching bash behavior)
pub fn list_clients() -> Result<(), String> {
    // Step 1: Load WireGuard parameters to get SERVER_WG_NIC
    let params = load_wireguard_params()
        .map_err(|e| format!("Failed to load WireGuard configuration: {}", e))?;

    // Step 2: Read server configuration file
    let config_path = format!("/etc/wireguard/{}.conf", params.server_wg_nic);
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                format!("WireGuard configuration file not found at {}\nPlease ensure WireGuard server is properly configured.", config_path)
            },
            std::io::ErrorKind::PermissionDenied => {
                format!("Permission denied reading {}\nPlease run this program with appropriate privileges (sudo).", config_path)
            },
            _ => format!("Failed to read {}: {}", config_path, e),
        })?;

    // Step 3: Find all client entries using regex
    let client_regex = Regex::new(r"^### Client (.+)$")
        .map_err(|e| format!("Failed to compile regex pattern: {}", e))?;

    let clients: Vec<String> = config_content
        .lines()
        .filter_map(|line| {
            client_regex
                .captures(line)
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().trim().to_string())
        })
        .collect();

    // Step 4: Handle no clients case (exit like bash version)
    if clients.is_empty() {
        println!();
        println!("You have no existing clients!");
        std::process::exit(1);
    }

    // Step 5: Display enhanced client list
    println!();
    println!("📋 WireGuard Client List");
    println!();
    println!(
        "Found {} client{}:",
        clients.len(),
        if clients.len() == 1 { "" } else { "s" }
    );

    for (index, client_name) in clients.iter().enumerate() {
        println!("  {}) {}", index + 1, client_name);
    }
    println!();

    // Wait for user to view the list before clearing in the main menu
    wait_for_key_press_with_message("Press any key to return to the main menu...");

    Ok(())
}

/// Get list of existing WireGuard clients from server configuration
/// This function extracts client names from the server config file
fn get_existing_clients(params: &WireguardParams) -> Result<Vec<String>, String> {
    let config_path = format!("/etc/wireguard/{}.conf", params.server_wg_nic);
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                format!("WireGuard configuration file not found at {}\nPlease ensure WireGuard server is properly configured.", config_path)
            },
            std::io::ErrorKind::PermissionDenied => {
                format!("Permission denied reading {}\nPlease run this program with appropriate privileges (sudo).", config_path)
            },
            _ => format!("Failed to read {}: {}", config_path, e),
        })?;

    // Extract client names using regex
    let client_regex = Regex::new(r"^### Client (.+)$")
        .map_err(|e| format!("Failed to compile regex pattern: {}", e))?;

    let clients: Vec<String> = config_content
        .lines()
        .filter_map(|line| {
            client_regex
                .captures(line)
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().trim().to_string())
        })
        .collect();

    Ok(clients)
}

/// Interactive client selection using arrow keys
/// Returns the selected client name
fn interactive_client_selection(clients: &[String]) -> Result<String, String> {
    println!();
    println!("Select the client you want to revoke:");

    let selection = Select::new()
        .items(clients)
        .with_prompt("Choose client to revoke")
        .default(0)
        .interact()
        .map_err(|e| format!("Selection error: {}", e))?;

    Ok(clients[selection].clone())
}

/// Confirm revocation with safety warning
/// Returns true if user confirms, false if cancelled
fn confirm_revocation(client_name: &str) -> Result<bool, String> {
    println!();
    println!("⚠️  WARNING: This action cannot be undone!");
    println!(
        "   Client '{}' will lose VPN access immediately.",
        client_name
    );

    let confirmed = Confirm::new()
        .with_prompt(&format!(
            "Are you sure you want to revoke '{}'?",
            client_name
        ))
        .default(false)
        .interact()
        .map_err(|e| format!("Confirmation error: {}", e))?;

    Ok(confirmed)
}

/// Remove client configuration from server config file
/// Uses smart parsing instead of sed for safer removal
fn remove_client_from_config(config_path: &str, client_name: &str) -> Result<(), String> {
    // Read current config
    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read server config: {}", e))?;

    // Parse and filter out client section
    let mut new_content = String::new();
    let mut lines = content.lines().peekable();
    let mut skip_block = false;

    while let Some(line) = lines.next() {
        // Check for client block start
        if line == format!("### Client {}", client_name) {
            skip_block = true;
            continue;
        }

        // Skip until empty line (end of peer block)
        if skip_block {
            if line.trim().is_empty() {
                skip_block = false;
                // Don't add the empty line that ends the block
                continue;
            }
            continue;
        }

        // Keep non-client lines
        new_content.push_str(line);
        new_content.push('\n');
    }

    // Write back the modified content
    fs::write(config_path, new_content.trim_end())
        .map_err(|e| format!("Failed to write updated server config: {}", e))?;

    Ok(())
}

/// Remove client configuration files from filesystem
/// Uses existing get_home_dir_for_client function for consistency
fn remove_client_files(client_name: &str, server_wg_nic: &str) -> Result<(), String> {
    // Get home directory (reuse existing function)
    let home_dir = get_home_dir_for_client(client_name)?;

    // Build client config file path
    let client_config_path =
        home_dir.join(format!("{}-client-{}.conf", server_wg_nic, client_name));

    // Remove client config file if it exists
    if client_config_path.exists() {
        fs::remove_file(&client_config_path).map_err(|e| {
            format!(
                "Failed to remove client config {}: {}",
                client_config_path.display(),
                e
            )
        })?;

        println!(
            "   ✓ Removed client configuration file: {}",
            client_config_path.display()
        );
    } else {
        println!("   ℹ️  Client configuration file not found (may have been removed already)");
    }

    Ok(())
}

/// Revoke an existing WireGuard client with interactive selection
/// Equivalent to the bash revokeClient() function with enhanced UX
///
/// This function:
/// 1. Loads WireGuard parameters and checks for existing clients
/// 2. Presents an interactive arrow-key selection interface
/// 3. Confirms revocation with safety warning
/// 4. Removes client from server configuration (best effort)
/// 5. Removes client configuration files (best effort)
/// 6. Syncs WireGuard service to apply changes
/// 7. Provides detailed success/failure feedback
pub fn revoke_client() -> Result<(), String> {
    // Step 1: Load WireGuard configuration
    let params = load_wireguard_params()
        .map_err(|e| format!("Failed to load WireGuard configuration: {}", e))?;

    // Step 2: Get existing clients
    let clients = get_existing_clients(&params)
        .map_err(|e| format!("Failed to retrieve client list: {}", e))?;

    // Step 3: Check if any clients exist
    if clients.is_empty() {
        println!();
        println!("You have no existing clients!");
        std::process::exit(1);
    }

    // Step 4: Display header
    println!();
    println!("🗑️  Revoke WireGuard Client");
    println!();
    println!(
        "Found {} client{}:",
        clients.len(),
        if clients.len() == 1 { "" } else { "s" }
    );

    for (index, client_name) in clients.iter().enumerate() {
        println!("  {}) {}", index + 1, client_name);
    }

    // Step 5: Interactive client selection
    let client_name = interactive_client_selection(&clients)?;

    // Step 6: Confirm revocation
    if !confirm_revocation(&client_name)? {
        println!();
        println!("Revocation cancelled.");
        return Ok(());
    }

    // Step 7: Best effort cleanup - collect errors but continue
    let mut errors = Vec::new();
    let mut successes = Vec::new();

    // Remove from server config
    let server_config_path = format!("/etc/wireguard/{}.conf", params.server_wg_nic);
    match remove_client_from_config(&server_config_path, &client_name) {
        Ok(()) => {
            successes.push("Removed from server configuration".to_string());
        }
        Err(e) => {
            errors.push(format!("Failed to remove from server config: {}", e));
        }
    }

    // Remove client files
    match remove_client_files(&client_name, &params.server_wg_nic) {
        Ok(()) => {
            successes.push("Client configuration file deleted".to_string());
        }
        Err(e) => {
            errors.push(format!("Failed to remove client files: {}", e));
        }
    }

    // Sync WireGuard service
    match sync_wireguard_config(&params.server_wg_nic) {
        Ok(()) => {
            successes.push("WireGuard service updated".to_string());
        }
        Err(e) => {
            errors.push(format!("Failed to sync WireGuard service: {}", e));
        }
    }

    // Report results
    println!();
    if errors.is_empty() {
        println!("✅ Client '{}' has been successfully revoked!", client_name);
        for success in successes {
            println!("   • {}", success);
        }
    } else if successes.is_empty() {
        println!("❌ Failed to revoke client '{}':", client_name);
        for error in errors {
            println!("   • {}", error);
        }
        return Err("Revocation failed completely".to_string());
    } else {
        println!("⚠️  Client '{}' partially revoked:", client_name);
        for success in successes {
            println!("   ✓ {}", success);
        }
        for error in &errors {
            println!("   ✗ {}", error);
        }
        println!();
        println!("Some operations failed, but the client may still lose access.");
        println!("Please check the errors above and resolve them manually if needed.");
    }

    println!();

    // Wait for user to see the result before clearing in the main menu
    wait_for_key_press_with_message("Press any key to return to the main menu...");

    Ok(())
}
