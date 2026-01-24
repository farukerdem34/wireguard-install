use crate::client::new_client;
use crate::enums::OsType;
use crate::models::InstallAnswers;
use crate::utils::{clear_terminal, set_permissions_recursive};
use dialoguer::{Confirm, Input};
use netwatcher;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::Ipv4Addr;
use std::path::Path;
use std::process;

/// Generate a WireGuard private key using the `wg genkey` command
fn generate_wg_private_key() -> Result<String, String> {
    let output = process::Command::new("wg")
        .arg("genkey")
        .output()
        .map_err(|e| format!("Failed to execute wg genkey: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "wg genkey failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let private_key = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to parse private key: {}", e))?
        .trim()
        .to_string();

    Ok(private_key)
}

/// Generate a WireGuard public key from a private key using the `wg pubkey` command
fn generate_wg_public_key(private_key: &str) -> Result<String, String> {
    let mut child = process::Command::new("wg")
        .arg("pubkey")
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute wg pubkey: {}", e))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(private_key.as_bytes())
            .map_err(|e| format!("Failed to write to wg pubkey stdin: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for wg pubkey: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "wg pubkey failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let public_key = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to parse public key: {}", e))?
        .trim()
        .to_string();

    Ok(public_key)
}

/// Create WireGuard private and public key pair using std::process::Command
/// Returns a tuple of (private_key, public_key)
pub fn create_pub_priv_keys() -> Result<(String, String), String> {
    // Generate private key using wg genkey command
    let private_key =
        generate_wg_private_key().map_err(|e| format!("Failed to generate private key: {}", e))?;

    // Generate public key from private key using wg pubkey command
    let public_key = generate_wg_public_key(&private_key)
        .map_err(|e| format!("Failed to generate public key: {}", e))?;

    Ok((private_key, public_key))
}

/// Detect the server's public IP address using external services
fn detect_public_ip() -> Result<String, String> {
    println!("Detecting server's public IP address...");

    // List of reliable IP detection services to try
    let services = [
        "https://ipv4.icanhazip.com",
        "https://api.ipify.org",
        "https://checkip.amazonaws.com",
    ];

    // Try each service with curl command
    for service in &services {
        let output = process::Command::new("curl")
            .args(["-s", "--connect-timeout", "10", service])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let ip = String::from_utf8(output.stdout)
                    .map_err(|e| format!("Failed to parse IP response: {}", e))?
                    .trim()
                    .to_string();

                // Basic validation - check if it looks like an IPv4 address
                if ip.parse::<Ipv4Addr>().is_ok() && !ip.is_empty() {
                    println!("Detected public IP: {}", ip);
                    return Ok(ip);
                }
            }
        }
    }

    Err("Failed to detect public IP from all services".to_string())
}

/// Check if a network interface exists using netwatcher
fn interface_exists(name: &str) -> bool {
    if let Ok(interfaces) = netwatcher::list_interfaces() {
        return interfaces.iter().any(|(_, iface)| iface.name == name);
    }
    false
}

/// Find the default route interface using system commands
fn find_default_route_interface() -> Result<String, String> {
    // Try route command first (more widely available)
    if let Ok(output) = process::Command::new("route")
        .args(["-n", "get", "default"])
        .output()
    {
        if output.status.success() {
            let route_output = String::from_utf8_lossy(&output.stdout);
            // Look for "interface: " line in route output
            for line in route_output.lines() {
                if line.trim().starts_with("interface:") {
                    let interface = line.split_whitespace().nth(1).unwrap_or("").trim();
                    if !interface.is_empty() && interface_exists(interface) {
                        return Ok(interface.to_string());
                    }
                }
            }
        }
    }

    // Try netstat as alternative
    if let Ok(output) = process::Command::new("netstat").args(["-rn"]).output() {
        if output.status.success() {
            let netstat_output = String::from_utf8_lossy(&output.stdout);
            // Look for default route (0.0.0.0 or 0/0)
            for line in netstat_output.lines() {
                if line.starts_with("0.0.0.0") || line.starts_with("default") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 6 {
                        // Interface is typically the last field in route table
                        let interface = parts[parts.len() - 1];
                        if interface_exists(interface) {
                            return Ok(interface.to_string());
                        }
                    }
                }
            }
        }
    }

    Err("Could not determine default route interface".to_string())
}

/// Detect the primary network interface used for internet connectivity
fn detect_primary_interface() -> Result<String, String> {
    println!("Detecting primary network interface...");

    // First try to find the actual default route interface
    if let Ok(interface) = find_default_route_interface() {
        println!("Detected interface via default route: {}", interface);
        return Ok(interface);
    }

    // Fallback: try common interface names in priority order
    let candidates = ["ens3", "ens5", "enp0s3", "eth0", "ens160", "wlan0"];

    println!("Checking common interface names...");
    for &name in &candidates {
        if interface_exists(name) {
            println!("Found existing interface: {}", name);
            return Ok(name.to_string());
        }
    }

    Err("Could not detect any suitable network interface".to_string())
}

/// Enum to represent the detected firewall system
#[derive(Debug, Clone, Copy)]
enum FirewallType {
    Firewalld,
    Iptables,
}

/// Detect which firewall system is running on the server
fn detect_firewall_system() -> FirewallType {
    let output = process::Command::new("pgrep").arg("firewalld").output();

    match output {
        Ok(output) if output.status.success() => {
            println!("Detected firewalld running");
            FirewallType::Firewalld
        }
        _ => {
            println!("Firewalld not detected, using iptables");
            FirewallType::Iptables
        }
    }
}

/// Calculate the network address from an IPv4 address
/// For example: 10.19.11.1 becomes 10.19.11.0
fn calculate_ipv4_network(ip: &Ipv4Addr) -> String {
    let octets = ip.octets();
    format!("{}.{}.{}.0", octets[0], octets[1], octets[2])
}

/// Generate firewalld PostUp and PostDown rules
fn generate_firewalld_rules(answers: &InstallAnswers) -> String {
    let network_ipv4 = calculate_ipv4_network(&answers.server_wg_ip);

    format!(
        "PostUp = firewall-cmd --zone=public --add-interface={} && firewall-cmd --add-port {}/udp && firewall-cmd --add-rich-rule='rule family=ipv4 source address={}/24 masquerade'
PostDown = firewall-cmd --zone=public --remove-interface={} && firewall-cmd --remove-port {}/udp && firewall-cmd --remove-rich-rule='rule family=ipv4 source address={}/24 masquerade'",
        answers.server_wg_nic,
        answers.server_wg_port,
        network_ipv4,
        answers.server_wg_nic,
        answers.server_wg_port,
        network_ipv4
    )
}

/// Generate iptables PostUp and PostDown rules
fn generate_iptables_rules(answers: &InstallAnswers) -> String {
    format!(
        "PostUp = iptables -I INPUT -p udp --dport {} -j ACCEPT
PostUp = iptables -I FORWARD -i {} -o {} -j ACCEPT
PostUp = iptables -I FORWARD -i {} -j ACCEPT
PostUp = iptables -t nat -A POSTROUTING -o {} -j MASQUERADE
PostDown = iptables -D INPUT -p udp --dport {} -j ACCEPT
PostDown = iptables -D FORWARD -i {} -o {} -j ACCEPT
PostDown = iptables -D FORWARD -i {} -j ACCEPT
PostDown = iptables -t nat -D POSTROUTING -o {} -j MASQUERADE",
        answers.server_wg_port,
        answers.server_public_nic,
        answers.server_wg_nic,
        answers.server_wg_nic,
        answers.server_public_nic,
        answers.server_wg_port,
        answers.server_public_nic,
        answers.server_wg_nic,
        answers.server_wg_nic,
        answers.server_public_nic
    )
}

/// Create the complete WireGuard server configuration file
fn create_wireguard_config(answers: &InstallAnswers) {
    let config_path = format!("/etc/wireguard/{}.conf", answers.server_wg_nic);

    println!("Creating WireGuard configuration file: {}", config_path);

    // Create the interface section
    let interface_config = format!(
        "[Interface]
Address = {}/24
ListenPort = {}
PrivateKey = {}
",
        answers.server_wg_ip, answers.server_wg_port, answers.server_priv_key
    );

    // Write the interface configuration
    fs::write(&config_path, interface_config)
        .expect("Failed to write WireGuard interface configuration");

    // Detect firewall system and generate appropriate rules
    let firewall_type = detect_firewall_system();
    let firewall_rules = match firewall_type {
        FirewallType::Firewalld => generate_firewalld_rules(answers),
        FirewallType::Iptables => generate_iptables_rules(answers),
    };

    // Append firewall rules to the configuration file
    let mut file = OpenOptions::new()
        .append(true)
        .open(&config_path)
        .expect("Failed to open WireGuard configuration file for appending");

    writeln!(file, "{}", firewall_rules)
        .expect("Failed to write firewall rules to WireGuard configuration");

    // Set secure permissions on the config file
    let config_file_path = Path::new(&config_path);
    set_permissions_recursive(config_file_path)
        .expect("Failed to set permissions on WireGuard configuration file");

    println!("WireGuard configuration file created successfully!");
}

/// Enable IP forwarding by creating sysctl configuration
fn enable_ip_routing() {
    println!("Enabling IP routing...");
    let sysctl_content = "net.ipv4.ip_forward = 1\nnet.ipv6.conf.all.forwarding = 1\n";

    fs::write("/etc/sysctl.d/wg.conf", sysctl_content)
        .expect("Failed to write sysctl configuration to /etc/sysctl.d/wg.conf");

    println!("IP forwarding enabled in /etc/sysctl.d/wg.conf");
}

/// Set Fedora-specific restrictive permissions on WireGuard directory and files
fn set_fedora_permissions() {
    println!("Setting Fedora-specific permissions on /etc/wireguard...");

    // Set directory permissions to 700
    let status = process::Command::new("chmod")
        .args(["-v", "700", "/etc/wireguard"])
        .status()
        .expect("Failed to set directory permissions on /etc/wireguard");

    if !status.success() {
        eprintln!("Warning: Failed to set directory permissions");
    }

    // Set file permissions to 600 for all files in the directory
    let status = process::Command::new("sh")
        .args(["-c", "chmod -v 600 /etc/wireguard/*"])
        .status()
        .expect("Failed to set file permissions on /etc/wireguard files");

    if !status.success() {
        eprintln!("Warning: Failed to set file permissions");
    }

    println!("Fedora-specific permissions applied to /etc/wireguard");
}

/// Configure WireGuard service for Alpine Linux (OpenRC)
fn configure_alpine_service(server_wg_nic: &str) {
    println!("Configuring WireGuard service for Alpine Linux...");

    // Apply sysctl configuration immediately
    let status = process::Command::new("sysctl")
        .args(["-p", "/etc/sysctl.d/wg.conf"])
        .status()
        .expect("Failed to apply sysctl configuration");

    if !status.success() {
        eprintln!("Warning: Failed to apply sysctl configuration");
    }

    // Add sysctl to boot services
    let status = process::Command::new("rc-update")
        .args(["add", "sysctl"])
        .status()
        .expect("Failed to add sysctl to boot services");

    if !status.success() {
        eprintln!("Warning: Failed to add sysctl to boot services");
    }

    // Create service symlink
    let symlink_target = format!("/etc/init.d/wg-quick.{}", server_wg_nic);
    let status = process::Command::new("ln")
        .args(["-s", "/etc/init.d/wg-quick", &symlink_target])
        .status()
        .expect("Failed to create service symlink");

    if !status.success() {
        eprintln!("Warning: Failed to create service symlink");
    }

    // Start the WireGuard service
    let service_name = format!("wg-quick.{}", server_wg_nic);
    let status = process::Command::new("rc-service")
        .args([&service_name, "start"])
        .status()
        .expect("Failed to start WireGuard service");

    if !status.success() {
        eprintln!("Warning: Failed to start WireGuard service");
    }

    // Enable service at boot
    let status = process::Command::new("rc-update")
        .args(["add", &service_name])
        .status()
        .expect("Failed to enable WireGuard service at boot");

    if !status.success() {
        eprintln!("Warning: Failed to enable WireGuard service at boot");
    }

    println!("WireGuard service configured and started for Alpine Linux");
}

/// Configure WireGuard service for systemd-based distributions
fn configure_systemd_service(server_wg_nic: &str) {
    println!("Configuring WireGuard service with systemd...");

    // Reload all sysctl configurations
    let status = process::Command::new("sysctl")
        .arg("--system")
        .status()
        .expect("Failed to reload sysctl configuration");

    if !status.success() {
        eprintln!("Warning: Failed to reload sysctl configuration");
    }

    // Start the WireGuard service
    let service_name = format!("wg-quick@{}", server_wg_nic);
    let status = process::Command::new("systemctl")
        .args(["start", &service_name])
        .status()
        .expect("Failed to start WireGuard service");

    if !status.success() {
        eprintln!("Warning: Failed to start WireGuard service");
    } else {
        println!("WireGuard service '{}' started successfully", service_name);
    }

    // Enable service at boot
    let status = process::Command::new("systemctl")
        .args(["enable", &service_name])
        .status()
        .expect("Failed to enable WireGuard service at boot");

    if !status.success() {
        eprintln!("Warning: Failed to enable WireGuard service at boot");
    } else {
        println!("WireGuard service '{}' enabled at boot", service_name);
    }

    println!("WireGuard service configured with systemd");
}

/// Configure and start WireGuard service based on the operating system type
fn configure_wireguard_service(os: OsType, server_wg_nic: &str) {
    println!("Configuring WireGuard service for OS: {:?}", os);

    match os {
        OsType::Fedora => {
            // Set restrictive permissions for Fedora
            set_fedora_permissions();
            // Use systemd for service management
            configure_systemd_service(server_wg_nic);
        }
        OsType::Alpine => {
            // Alpine uses OpenRC instead of systemd
            configure_alpine_service(server_wg_nic);
        }
        // Most other distributions use systemd
        OsType::Debian
        | OsType::Ubuntu
        | OsType::Raspbian
        | OsType::Centos
        | OsType::AlmaLinux
        | OsType::Rocky
        | OsType::Oracle
        | OsType::Arch => {
            configure_systemd_service(server_wg_nic);
        }
        OsType::Unknown => {
            println!("Unknown OS type - attempting systemd service configuration");
            configure_systemd_service(server_wg_nic);
        }
    }

    println!("WireGuard service configuration completed");
}

/// Write InstallAnswers to /etc/wireguard/params file in the specified format
fn write_params_file(answers: &InstallAnswers) -> Result<(), String> {
    let params_content = format!(
        "SERVER_PUB_IP={}\nSERVER_PUB_NIC={}\nSERVER_WG_NIC={}\nSERVER_WG_IPV4={}\nSERVER_WG_IPV6={}\nSERVER_PORT={}\nSERVER_PRIV_KEY={}\nSERVER_PUB_KEY={}\nCLIENT_DNS_1={}\nCLIENT_DNS_2={}\nALLOWED_IPS={}",
        answers.server_pub_ip,
        answers.server_public_nic,
        answers.server_wg_nic,
        answers.server_wg_ip,
        answers.server_pub_ipv6.as_ref().unwrap_or(&"".to_string()),
        answers.server_wg_port,
        answers.server_priv_key,
        answers.server_pub_key,
        answers.client_dns_1,
        answers.client_dns_2,
        answers.allowed_ips
    );

    fs::write("/etc/wireguard/params", params_content)
        .map_err(|e| format!("Failed to write params file: {}", e))?;

    // Set secure permissions (600) on the params file since it contains private keys
    let params_path = Path::new("/etc/wireguard/params");
    set_permissions_recursive(params_path)
        .map_err(|e| format!("Failed to set permissions on params file: {}", e))?;

    Ok(())
}

pub fn install_wireguard(os: OsType) {
    let mut answers: InstallAnswers = install_question();
    let cmds = match os {
        OsType::Debian | OsType::Ubuntu | OsType::Raspbian => vec![
            "apt-get update",
            "apt-get install -y wireguard iptables resolvconf qrencode",
        ],
        OsType::Fedora => {
            if std::env::var("VERSION_ID")
                .expect("Failed to get version ID environment variable")
                .parse::<u8>()
                .expect("Failed to parse Fedora version ID")
                > 32
            {
                vec![
                    "dnf install -y dnf-plugins-core",
                    "dnf copr enable -y jdoss/wireguard",
                    "dnf install -y wireguard-dkms",
                ]
            } else {
                vec!["dnf install -y wireguard-tools iptables qrencode"]
            }
        }
        OsType::Centos | OsType::AlmaLinux | OsType::Rocky => {
            let version_id: String =
                std::env::var("VERSION_ID").expect("Failed to get version ID environment variable");
            if version_id.starts_with("8") {
                vec![
                    "yum install -y epel-release elrepo-release",
                    "yum install -y kmod-wireguard",
                    "qrencode",
                ]
            } else {
                vec!["yum install -y wireguard-tools iptables"]
            }
        }
        OsType::Oracle => vec!["yum install -y wireguard-tools iptables qrencode"],
        OsType::Arch => vec!["pacman -S --needed --noconfirm wireguard-tools qrencode"],
        OsType::Alpine => vec!["apk add wireguard-tools iptables libqrencode-tools"],
        OsType::Unknown => {
            eprintln!("Unrecognized OS");
            process::exit(1)
        }
    };
    for cmd in cmds {
        println!("Installing packages: {}", cmd);
        let status = process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()
            .expect("Failed to execute package installation command");

        if !status.success() {
            eprintln!("Package installation failed for command: {}", cmd);
            eprintln!("WireGuard installation incomplete. Please install WireGuard manually and try again.");
            process::exit(1);
        }
    }
    // Verify WireGuard installation by checking for the wg command
    println!("Verifying WireGuard installation...");
    let wg_check = process::Command::new("sh")
        .args(["-c", "command -v wg"])
        .output()
        .expect("Failed to check for wg command");

    if !wg_check.status.success() {
        eprintln!("Error: 'wg' command not found after installation.");
        eprintln!("WireGuard may not have been installed correctly.");
        eprintln!("Please verify your package manager and try installing WireGuard manually:");
        eprintln!("  - For Debian/Ubuntu: apt-get install wireguard");
        eprintln!("  - For Fedora: dnf install wireguard-tools");
        eprintln!("  - For CentOS/RHEL: yum install wireguard-tools");
        process::exit(1);
    }

    // Additional verification: try to run wg with --version to ensure it's working
    let wg_version_check = process::Command::new("wg").arg("--version").output();

    match wg_version_check {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("WireGuard verification successful: {}", version.trim());
        }
        Ok(_) => {
            eprintln!("Warning: 'wg' command exists but may not be functioning correctly.");
            eprintln!("Continuing with installation...");
        }
        Err(e) => {
            eprintln!("Error: Failed to execute 'wg --version': {}", e);
            eprintln!("This suggests WireGuard is not properly installed or accessible.");
            eprintln!("Please check your WireGuard installation and PATH environment variable.");
            process::exit(1);
        }
    }

    let _ = fs::create_dir("/etc/wireguard");
    set_permissions_recursive(Path::new("/etc/wireguard"))
        .expect("Failed to set permissions on /etc/wireguard");

    // Generate WireGuard server keys
    println!("Generating WireGuard server keys...");
    let (server_private_key, server_public_key) =
        create_pub_priv_keys().expect("Failed to generate WireGuard key pair");

    // Update answers with generated keys
    answers.server_priv_key = server_private_key;
    answers.server_pub_key = server_public_key;

    // Write configuration to params file
    println!("Writing WireGuard configuration to /etc/wireguard/params...");
    write_params_file(&answers).expect("Failed to write WireGuard parameters file");

    // Create the WireGuard server configuration file
    create_wireguard_config(&answers);

    // Enable IP routing
    enable_ip_routing();

    // Configure and start WireGuard service based on OS
    configure_wireguard_service(os, &answers.server_wg_nic);

    // Create first client
    if let Err(e) = new_client() {
        eprintln!("Warning: Failed to create initial client: {}", e);
        eprintln!("You can add clients later using the client management menu.");
    }

    println!("WireGuard installation and configuration completed successfully!");
    println!("If you want to add more clients, you simply need to run this script another time!");
    std::process::exit(0);
}

pub fn install_question() -> InstallAnswers {
    println!(
        r#"
    Welcome to the WireGuard installer!
    The git repository is available at: https://github.com/farukerdem34/wireguard-install

    I need to ask you a few questions before starting the setup.
    You can keep the default options and just press enter if you are ok with them.

    "#
    );
    let predicted_server_public_ip = detect_public_ip().unwrap_or_else(|err| {
        println!(
            "Could not auto-detect public IP ({}), using fallback: 203.0.113.1",
            err
        );
        "203.0.113.1".to_string()
    });
    let predicted_server_public_nic = detect_primary_interface().unwrap_or_else(|err| {
        println!(
            "Could not auto-detect network interface ({}), using fallback: eth0",
            err
        );
        "eth0".to_string()
    });
    let server_public_ip: String = Input::new()
        .with_prompt("IPv4 public address: ")
        .default(predicted_server_public_ip)
        .interact_text()
        .unwrap();
    let want_ipv6: bool = Confirm::new()
        .with_prompt("Do you want to set IPv6?")
        .default(false)
        .interact()
        .unwrap();
    let mut server_public_ipv6: Option<String> = None;
    if want_ipv6 {
        let predicted_server_public_ipv6 = "::1".to_string(); // mocked prediction
        server_public_ipv6 = Some(
            Input::new()
                .with_prompt("IPv6 public address: ")
                .default(predicted_server_public_ipv6)
                .interact_text()
                .unwrap(),
        );
    }
    let server_public_nic: String = Input::new()
        .with_prompt("Public interface: ")
        .default(predicted_server_public_nic)
        .interact_text()
        .unwrap();
    let server_wg_interface: String = Input::new()
        .with_prompt("WireGuard interface name: ")
        .default("wg0".to_string())
        .interact_text()
        .unwrap();
    let server_wg_ip: String = Input::new()
        .with_prompt("Server WireGuard IPv4: ")
        .default("10.19.11.1".to_string())
        .validate_with(|ip: &String| {
            ip.parse::<Ipv4Addr>()
                .map(|_| ())
                .map_err(|_| "Invalid IPv4 address")
        })
        .interact_text()
        .unwrap();
    let server_port: String = Input::new()
        .with_prompt("Server port: ")
        .default("51820".to_string())
        .interact_text()
        .unwrap();
    let client_dns_1: String = Input::new()
        .with_prompt("DNS 1: ")
        .default("1.1.1.1".to_string())
        .validate_with(|ip: &String| {
            ip.parse::<Ipv4Addr>()
                .map(|_| ())
                .map_err(|_| "Invalid IPv4 address")
        })
        .interact_text()
        .unwrap();
    let client_dns_2: String = Input::new()
        .with_prompt("DNS 2: ")
        .default("1.0.0.1".to_string())
        .validate_with(|ip: &String| {
            ip.parse::<Ipv4Addr>()
                .map(|_| ())
                .map_err(|_| "Invalid IPv4 address")
        })
        .interact_text()
        .unwrap();
    let allowed_ips: String = Input::new()
        .with_prompt(
            r#"
WireGuard uses a parameter called AllowedIPs to determine what is routed over the VPN.
Allowed IPs list for generated clients (leave default to route everything):
        "#,
        )
        .default("0.0.0.0/0".to_string())
        .interact_text()
        .unwrap();

    // Clear terminal after installation questions are complete
    clear_terminal();

    InstallAnswers {
        server_pub_ip: server_public_ip
            .parse::<Ipv4Addr>()
            .expect("Failed to parse public IPv4 address"),
        server_public_nic: server_public_nic,
        server_pub_ipv6: server_public_ipv6,
        server_wg_ip: server_wg_ip
            .parse::<Ipv4Addr>()
            .expect("Failed to parse wg0 IP"),
        server_wg_nic: server_wg_interface,
        server_wg_port: server_port.parse::<u16>().expect("Failed to parse port"),
        server_priv_key: String::new(), // Will be filled later
        server_pub_key: String::new(),  // Will be filled later
        client_dns_1: client_dns_1
            .parse::<Ipv4Addr>()
            .expect("Failed to parse DNS 1"),
        client_dns_2: client_dns_2
            .parse::<Ipv4Addr>()
            .expect("Failed to parse DNS 2"),
        allowed_ips,
    }
}
