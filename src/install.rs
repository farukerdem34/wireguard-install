use crate::enums::OsType;
use crate::models::InstallAnswers;
use crate::utils::set_permissions_recursive;
use dialoguer::Input;
use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;
use std::process;

pub fn install_wireguard(os: OsType) {
    let _answers: InstallAnswers = install_question();
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
        client_dns_1: client_dns_1
            .parse::<Ipv4Addr>()
            .expect("Failed to parse DNS 1"),
        client_dns_2: client_dns_2
            .parse::<Ipv4Addr>()
            .expect("Failed to parse DNS 2"),
        allowed_ips,
    }
}
