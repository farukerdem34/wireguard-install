use crate::enums::OsType;
use crate::models::InstallAnswers;
use crate::utils::set_permissions_recursive;
use dialoguer::Input;
use std::fs;
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
        OsType::Debian | OsType::Ubuntu | OsType::Rasbian => vec![
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
        OsType::Arch => vec!["pacman -S --needed --noconfirm wireguard-tools qrencode"],
        OsType::Alpine => vec!["apk add wireguard-tools iptables libqrencode-tools"],
        OsType::Unknown => {
            eprintln!("Unrecognized OS");
            process::exit(1)
        }
    };
    for cmd in cmds {
        let words = cmd.split_whitespace().collect::<Vec<&str>>();
        let _ = process::Command::new("sh")
            .arg("-c")
            .args(words)
            .stderr(process::Stdio::null())
            .stdout(process::Stdio::null())
            .status();
    }
    if !process::Command::new("sh")
        .args(vec!["-c", "command", "-v", "wg"])
        .status()
        .expect("failed to find wg command")
        .success()
    {
        eprintln!("WireGuard couldn't be installed successfully. Exiting...");
        process::exit(1);
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

    println!("WireGuard installation and configuration completed successfully!");
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
    let predicted_server_public_ip = "192.168.1.1".to_string(); // mocked prediction
    let predicted_server_public_nic = "eth0".to_string(); // mocked NIC
    let server_public_ip: String = Input::new()
        .with_prompt("IPv4 public address: ")
        .default(predicted_server_public_ip)
        .interact_text()
        .unwrap();
    let want_ipv6: bool = Input::new()
        .with_prompt("Do you want to set IPv6?")
        .default(false)
        .interact_text()
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
