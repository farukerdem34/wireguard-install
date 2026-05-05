use crate::models::{InterfaceConfig, MultiInterfaceConfig};
use crate::network::{
    detect_subnet_conflicts, get_server_ip_from_subnet, is_port_in_use, suggest_available_port,
    validate_subnet,
};
use dialoguer::{Confirm, Input, Select};
use std::fs;
use std::path::Path;
use std::process::Command;

const MULTI_INTERFACE_CONFIG_PATH: &str = "/etc/wireguard/interfaces.json";

fn ensure_wireguard_directory() -> Result<(), String> {
    let dir_path = Path::new("/etc/wireguard");

    if !dir_path.exists() {
        fs::create_dir_all(dir_path)
            .map_err(|e| format!("Failed to create /etc/wireguard directory: {}", e))?;
    }

    Ok(())
}

pub fn load_multi_interface_config() -> Result<MultiInterfaceConfig, String> {
    if Path::new(MULTI_INTERFACE_CONFIG_PATH).exists() {
        let content = fs::read_to_string(MULTI_INTERFACE_CONFIG_PATH)
            .map_err(|e| format!("Failed to read interfaces config: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse interfaces config: {}", e))
    } else {
        Ok(MultiInterfaceConfig::new())
    }
}

pub fn save_multi_interface_config(config: &MultiInterfaceConfig) -> Result<(), String> {
    ensure_wireguard_directory()?;

    let json_content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(MULTI_INTERFACE_CONFIG_PATH, json_content)
        .map_err(|e| format!("Failed to write interfaces config: {}", e))?;

    Ok(())
}

pub fn create_new_interface() -> Result<(), String> {
    println!("🔧 Creating new WireGuard interface");

    ensure_wireguard_directory()?;

    let mut config = load_multi_interface_config()?;

    // Suggest interface name
    let suggested_name = config.get_next_interface_name();
    let interface_name: String = Input::new()
        .with_prompt("Interface name")
        .default(suggested_name)
        .interact_text()
        .map_err(|e| format!("Failed to get interface name: {}", e))?;

    // Validate interface name
    if config.interfaces.contains_key(&interface_name) {
        return Err(format!("Interface {} already exists", interface_name));
    }

    if !interface_name.starts_with("wg") {
        return Err("Interface name must start with 'wg' (e.g., wg0, wg1, wg2)".to_string());
    }

    // Get subnet
    let subnet: String = Input::new()
        .with_prompt("Subnet (CIDR format, e.g., 10.19.23.0/24)")
        .interact_text()
        .map_err(|e| format!("Failed to get subnet: {}", e))?;

    // Validate subnet and check for conflicts
    detect_subnet_conflicts(&subnet, &config)?;
    let server_ip = get_server_ip_from_subnet(&subnet)?;

    // Suggest port
    let existing_ports: Vec<u16> = config.interfaces.values().map(|i| i.port).collect();
    let suggested_port = suggest_available_port(&existing_ports, config.get_next_suggested_port());

    let port_input: String = Input::new()
        .with_prompt("Port")
        .default(suggested_port.to_string())
        .interact_text()
        .map_err(|e| format!("Failed to get port: {}", e))?;

    let port: u16 = port_input.parse().map_err(|_| "Invalid port number")?;

    // Validate port
    if port < 1024 {
        return Err("Port must be between 1024 and 65535".to_string());
    }

    if existing_ports.contains(&port) {
        return Err(format!(
            "Port {} is already used by another interface",
            port
        ));
    }

    if is_port_in_use(port) {
        return Err(format!(
            "Port {} is already in use by another process",
            port
        ));
    }

    // Generate keys
    let (private_key, public_key) = generate_keys()?;

    // Create interface config
    let interface_config = InterfaceConfig {
        name: interface_name.clone(),
        subnet: subnet.clone(),
        server_ip,
        port,
        private_key: private_key.clone(),
        public_key: public_key.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        active: true,
    };

    // Show summary and confirm
    println!("\\n📋 Interface Summary:");
    println!("  Name: {}", interface_name);
    println!("  Subnet: {}", subnet);
    println!("  Server IP: {}", server_ip);
    println!("  Port: {}", port);
    println!("  Public Key: {}", public_key);

    let confirm = Confirm::new()
        .with_prompt("Create this interface?")
        .default(true)
        .interact()
        .map_err(|e| format!("Failed to get confirmation: {}", e))?;

    if !confirm {
        return Ok(());
    }

    // Create WireGuard configuration file
    create_interface_config_file(&interface_config)?;

    // Enable and start the interface
    enable_interface(&interface_name)?;

    // Update and save configuration
    config
        .interfaces
        .insert(interface_name.clone(), interface_config);
    config.next_suggested_port = port + 1;
    save_multi_interface_config(&config)?;

    println!("✅ Interface {} created successfully!", interface_name);
    Ok(())
}

pub fn list_interfaces() -> Result<(), String> {
    let config = load_multi_interface_config()?;

    if config.interfaces.is_empty() {
        println!("No WireGuard interfaces configured.");
        return Ok(());
    }

    println!("📡 WireGuard Interfaces:");
    println!();

    for (name, interface) in &config.interfaces {
        let status = if interface.active {
            "🟢 Active"
        } else {
            "🔴 Inactive"
        };
        let client_count = count_interface_clients(name)?;

        println!("🔸 Interface: {}", name);
        println!("  Status: {}", status);
        println!("  Subnet: {}", interface.subnet);
        println!("  Server IP: {}", interface.server_ip);
        println!("  Port: {}", interface.port);
        println!("  Clients: {}", client_count);
        println!("  Created: {}", interface.created_at);
        println!();
    }

    Ok(())
}

pub fn remove_interface() -> Result<(), String> {
    let mut config = load_multi_interface_config()?;

    if config.interfaces.is_empty() {
        return Err("No interfaces configured to remove".to_string());
    }

    let interface_names: Vec<String> = config.interfaces.keys().cloned().collect();

    let selection = Select::new()
        .with_prompt("Select interface to remove")
        .items(&interface_names)
        .interact()
        .map_err(|e| format!("Failed to select interface: {}", e))?;

    let interface_name = &interface_names[selection];
    let interface = config.interfaces.get(interface_name).unwrap();

    // Show interface info and confirm
    println!("\\n⚠️  Interface to remove:");
    println!("  Name: {}", interface.name);
    println!("  Subnet: {}", interface.subnet);
    println!("  Port: {}", interface.port);

    let client_count = count_interface_clients(interface_name)?;
    if client_count > 0 {
        println!(
            "  ⚠️  This interface has {} client(s) that will also be removed!",
            client_count
        );
    }

    let confirm = Confirm::new()
        .with_prompt("Are you sure you want to remove this interface?")
        .default(false)
        .interact()
        .map_err(|e| format!("Failed to get confirmation: {}", e))?;

    if !confirm {
        return Ok(());
    }

    // Stop and disable the interface
    disable_interface(interface_name)?;

    // Remove interface configuration file
    let config_path = format!("/etc/wireguard/{}.conf", interface_name);
    if Path::new(&config_path).exists() {
        fs::remove_file(&config_path)
            .map_err(|e| format!("Failed to remove config file {}: {}", config_path, e))?;
    }

    // Remove from configuration
    config.interfaces.remove(interface_name);
    save_multi_interface_config(&config)?;

    println!("✅ Interface {} removed successfully!", interface_name);
    Ok(())
}

fn generate_keys() -> Result<(String, String), String> {
    // Generate private key
    let private_key_output = Command::new("wg")
        .args(&["genkey"])
        .output()
        .map_err(|e| format!("Failed to generate private key: {}", e))?;

    if !private_key_output.status.success() {
        return Err("Failed to generate private key".to_string());
    }

    let private_key = String::from_utf8(private_key_output.stdout)
        .map_err(|e| format!("Invalid private key format: {}", e))?
        .trim()
        .to_string();

    // Generate public key from private key
    let mut public_key_cmd = Command::new("wg")
        .arg("pubkey")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start wg pubkey: {}", e))?;

    if let Some(stdin) = public_key_cmd.stdin.take() {
        let mut stdin = stdin;
        use std::io::Write;
        stdin
            .write_all(private_key.as_bytes())
            .map_err(|e| format!("Failed to write to wg pubkey stdin: {}", e))?;
    }

    let public_key_output = public_key_cmd
        .wait_with_output()
        .map_err(|e| format!("Failed to get wg pubkey output: {}", e))?;

    if !public_key_output.status.success() {
        return Err("wg pubkey command failed".to_string());
    }

    let public_key = String::from_utf8(public_key_output.stdout)
        .map_err(|e| format!("Invalid public key format: {}", e))?
        .trim()
        .to_string();

    Ok((private_key, public_key))
}

fn create_interface_config_file(interface: &InterfaceConfig) -> Result<(), String> {
    let prefix = extract_prefix_from_subnet(&interface.subnet)?;
    let public_interface = get_public_interface()?;
    
    let config_content = format!(
        "[Interface]\nPrivateKey = {}\nAddress = {}/{}\nListenPort = {}\nPostUp = iptables -A FORWARD -i {} -j ACCEPT; iptables -A FORWARD -o {} -j ACCEPT; iptables -t nat -A POSTROUTING -o {} -j MASQUERADE\nPostDown = iptables -D FORWARD -i {} -j ACCEPT; iptables -D FORWARD -o {} -j ACCEPT; iptables -t nat -D POSTROUTING -o {} -j MASQUERADE\n",
        interface.private_key,
        interface.server_ip,
        prefix,
        interface.port,
        interface.name,
        interface.name,
        public_interface,
        interface.name,
        interface.name,
        public_interface
    );

    let config_path = format!("/etc/wireguard/{}.conf", interface.name);
    fs::write(&config_path, config_content)
        .map_err(|e| format!("Failed to write config file {}: {}", config_path, e))?;

    // Set proper permissions (600)
    use std::path::Path;
    crate::utils::set_permissions_recursive(Path::new(&config_path))
        .map_err(|e| format!("Failed to set permissions on {}: {}", config_path, e))?;

    Ok(())
}

fn extract_prefix_from_subnet(subnet: &str) -> Result<u8, String> {
    let parts: Vec<&str> = subnet.split('/').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid subnet format: {}", subnet));
    }

    parts[1]
        .parse::<u8>()
        .map_err(|_| format!("Invalid subnet prefix: {}", parts[1]))
}

pub fn get_public_interface() -> Result<String, String> {
    // Try to detect the public interface using default route
    let output = Command::new("ip")
        .args(&["route", "show", "default"])
        .output()
        .map_err(|e| format!("Failed to detect public interface: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("dev") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(dev_index) = parts.iter().position(|&x| x == "dev") {
                if let Some(interface) = parts.get(dev_index + 1) {
                    return Ok(interface.to_string());
                }
            }
        }
    }

    // Fallback to common interface names
    for interface in &["eth0", "ens3", "enp0s3", "wlan0"] {
        let check_output = Command::new("ip")
            .args(&["link", "show", interface])
            .output();

        if let Ok(output) = check_output {
            if output.status.success() {
                return Ok(interface.to_string());
            }
        }
    }

    Err("Could not detect public network interface".to_string())
}

fn enable_interface(interface_name: &str) -> Result<(), String> {
    // Try systemd first
    if Command::new("systemctl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        // Enable interface with systemd
        let enable_output = Command::new("systemctl")
            .args(&["enable", &format!("wg-quick@{}", interface_name)])
            .output()
            .map_err(|e| format!("Failed to enable interface {}: {}", interface_name, e))?;

        if !enable_output.status.success() {
            let stderr = String::from_utf8_lossy(&enable_output.stderr);
            return Err(format!(
                "Failed to enable interface {}: {}",
                interface_name, stderr
            ));
        }

        // Start interface with systemd
        let start_output = Command::new("systemctl")
            .args(&["start", &format!("wg-quick@{}", interface_name)])
            .output()
            .map_err(|e| format!("Failed to start interface {}: {}", interface_name, e))?;

        if !start_output.status.success() {
            let stderr = String::from_utf8_lossy(&start_output.stderr);
            return Err(format!(
                "Failed to start interface {}: {}",
                interface_name, stderr
            ));
        }

        println!("✅ Interface {} enabled and started (systemd)", interface_name);
        return Ok(());
    }

    // Try OpenRC (Alpine)
    if Command::new("rc-service")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        // Create symlink for OpenRC
        let symlink_path = format!("/etc/init.d/wg-quick.{}", interface_name);
        if !Path::new(&symlink_path).exists() {
            let ln_output = Command::new("ln")
                .args(["-s", "/etc/init.d/wg-quick", &symlink_path])
                .output();
            
            if let Ok(output) = ln_output {
                if !output.status.success() {
                    println!("Warning: Failed to create service symlink for {}", interface_name);
                }
            }
        }

        // Add to default runlevel
        let add_output = Command::new("rc-update")
            .args(["add", &format!("wg-quick.{}", interface_name)])
            .output()
            .map_err(|e| format!("Failed to enable interface {}: {}", interface_name, e))?;

        if !add_output.status.success() {
            println!(
                "Warning: Failed to add {} to runlevel",
                interface_name
            );
        }

        // Start the service
        let start_output = Command::new("rc-service")
            .args([&format!("wg-quick.{}", interface_name), "start"])
            .output()
            .map_err(|e| format!("Failed to start interface {}: {}", interface_name, e))?;

        if !start_output.status.success() {
            let stderr = String::from_utf8_lossy(&start_output.stderr);
            return Err(format!(
                "Failed to start interface {}: {}",
                interface_name, stderr
            ));
        }

        println!("✅ Interface {} enabled and started (OpenRC)", interface_name);
        return Ok(());
    }

    Err("No supported init system found (systemd or OpenRC required)".to_string())
}

fn disable_interface(interface_name: &str) -> Result<(), String> {
    // Try systemd first
    if Command::new("systemctl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        // Stop interface
        let stop_output = Command::new("systemctl")
            .args(&["stop", &format!("wg-quick@{}", interface_name)])
            .output()
            .map_err(|e| format!("Failed to stop interface {}: {}", interface_name, e))?;

        if !stop_output.status.success() {
            let stderr = String::from_utf8_lossy(&stop_output.stderr);
            println!(
                "Warning: Failed to stop interface {}: {}",
                interface_name, stderr
            );
        }

        // Disable interface
        let disable_output = Command::new("systemctl")
            .args(&["disable", &format!("wg-quick@{}", interface_name)])
            .output()
            .map_err(|e| format!("Failed to disable interface {}: {}", interface_name, e))?;

        if !disable_output.status.success() {
            let stderr = String::from_utf8_lossy(&disable_output.stderr);
            println!(
                "Warning: Failed to disable interface {}: {}",
                interface_name, stderr
            );
        }

        println!("✅ Interface {} disabled (systemd)", interface_name);
        return Ok(());
    }

    // Try OpenRC (Alpine)
    if Command::new("rc-service")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        // Stop the service
        let stop_output = Command::new("rc-service")
            .args([&format!("wg-quick.{}", interface_name), "stop"])
            .output()
            .map_err(|e| format!("Failed to stop interface {}: {}", interface_name, e))?;

        if !stop_output.status.success() {
            let stderr = String::from_utf8_lossy(&stop_output.stderr);
            println!(
                "Warning: Failed to stop interface {}: {}",
                interface_name, stderr
            );
        }

        // Remove from default runlevel
        let remove_output = Command::new("rc-update")
            .args(["del", &format!("wg-quick.{}", interface_name)])
            .output()
            .map_err(|e| format!("Failed to disable interface {}: {}", interface_name, e))?;

        if !remove_output.status.success() {
            println!(
                "Warning: Failed to remove {} from runlevel",
                interface_name
            );
        }

        // Remove symlink
        let symlink_path = format!("/etc/init.d/wg-quick.{}", interface_name);
        if Path::new(&symlink_path).exists() {
            let _ = fs::remove_file(&symlink_path);
        }

        println!("✅ Interface {} disabled (OpenRC)", interface_name);
        return Ok(());
    }

    println!("Warning: No supported init system found");
    Ok(())
}

fn count_interface_clients(interface_name: &str) -> Result<usize, String> {
    let config_path = format!("/etc/wireguard/{}.conf", interface_name);

    if !Path::new(&config_path).exists() {
        return Ok(0);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file {}: {}", config_path, e))?;

    let client_count = content.matches("### Client ").count();
    Ok(client_count)
}

pub fn select_interface_for_client() -> Result<String, String> {
    let config = load_multi_interface_config()?;

    if config.interfaces.is_empty() {
        return Err("No interfaces configured. Please create an interface first.".to_string());
    }

    let mut interface_options = Vec::new();
    let mut interface_names = Vec::new();

    for (name, interface) in &config.interfaces {
        if interface.active {
            let client_count = count_interface_clients(name)?;
            let subnet_size = calculate_subnet_size(&interface.subnet)?;
            let available = subnet_size - 2 - client_count; // -2 for network and broadcast

            interface_options.push(format!(
                "{} ({}) - Port: {} - Clients: {}/{} available",
                name, interface.subnet, interface.port, client_count, available
            ));
            interface_names.push(name.clone());
        }
    }

    if interface_options.is_empty() {
        return Err("No active interfaces available".to_string());
    }

    if interface_options.len() == 1 {
        return Ok(interface_names[0].clone());
    }

    let selection = Select::new()
        .with_prompt("Select interface for new client")
        .items(&interface_options)
        .interact()
        .map_err(|e| format!("Failed to select interface: {}", e))?;

    Ok(interface_names[selection].clone())
}

fn calculate_subnet_size(subnet: &str) -> Result<usize, String> {
    let network = validate_subnet(subnet)?;
    Ok(network.size() as usize)
}

pub fn get_interface_config(interface_name: &str) -> Result<InterfaceConfig, String> {
    let config = load_multi_interface_config()?;

    config
        .interfaces
        .get(interface_name)
        .cloned()
        .ok_or_else(|| format!("Interface {} not found", interface_name))
}
