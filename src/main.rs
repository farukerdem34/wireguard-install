use dialoguer::Input;
use rand::Rng;
use std::fs;
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
pub enum OsType {
    Ubuntu,
    Debian,
    Fedora,
    Centos,
    AlmaLinux,
    Rocky,
    Arch,
    Rasbian,
    Alpine,
    Unknown,
}
pub struct InstallAnswers {
    server_pub_ip: Ipv4Addr,
    server_public_nic: String,
    server_pub_ipv6: Option<String>,
    server_wg_nic: String,
    server_wg_ip: Ipv4Addr,
    server_wg_port: u16,
    client_dns_1: Ipv4Addr,
    client_dns_2: Ipv4Addr,
    allowed_ips: String,
}

#[tokio::main]
async fn main() {
    let _ = initial_check().await;
}
pub fn install_wireguard(os: OsType) {
    let answers: InstallAnswers = install_question();
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
            std::process::exit(1)
        }
    };
    for cmd in cmds {
        let words = cmd.split_whitespace().collect::<Vec<&str>>();
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .args(words)
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .status();
    }
    if !std::process::Command::new("sh")
        .args(vec!["-c", "command", "-v", "wg"])
        .status()
        .expect("failed to find wg command")
        .success()
    {
        eprintln!("Wireguard couldn't installed succesfully. Exiting...");
        std::process::exit(1);
    }

    let _ = fs::create_dir("/etc/wireguard");
    fn set_permissions_recursive(path: &Path) -> io::Result<()> {
        let metadata = fs::metadata(path)?;
        let mut permissions = metadata.permissions();
        if metadata.is_dir() {
            permissions.set_mode(0o700);
        } else {
            permissions.set_mode(0o600);
        }
        fs::set_permissions(path, permissions)?;

        if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                set_permissions_recursive(&entry_path)?;
            }
        }

        Ok(())
    }
    set_permissions_recursive(Path::new("/etc/wireguard"))
        .expect("Failed to set permissions on /etc/wireguard")
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
    let (predicted_server_public_ip, predicted_server_public_nic) = {
        let interfaces = netwatcher::list_interfaces().expect("Failed to list interfaces");
        let mut _ip = Some(String::new());
        let mut _interface = Some(String::new());
        for i in interfaces.values() {
            for ip in &i.ips {
                if ip.ip.to_string() != "127.0.0.1" {
                    _ip = Some(ip.ip.to_string());
                    _interface = Some(i.name.clone())
                }
            }
        }
        (_ip, _interface)
    };
    let server_public_ip: String = Input::new()
        .with_prompt("IPv4 public address: ")
        .default(predicted_server_public_ip.unwrap())
        .interact_text()
        .unwrap();
    let want_ipv6: bool = Input::new()
        .with_prompt("Do you want to set IPv6?")
        .default(false)
        .interact_text()
        .unwrap();
    let mut server_public_ipv6: Option<String> = None;
    if want_ipv6 {
        let predicted_server_public_ipv6 = {
            let ipv4 = Ipv4Addr::from_str(&server_public_ip).unwrap();
            let ipv6: Ipv6Addr = ipv4.to_ipv6_mapped();
            ipv6
        };
        server_public_ipv6 = Some(
            Input::new()
                .with_prompt("IPv6 public address: ")
                .default(predicted_server_public_ipv6.to_string())
                .interact_text()
                .unwrap(),
        );
    }
    let server_public_nic: String = Input::new()
        .with_prompt("Public interface: ")
        .default(predicted_server_public_nic.unwrap())
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
        .interact_text()
        .unwrap();
    // let mut rng = rand::rng();
    // let numbers: Vec<u16> = (50000..65000).collect();
    // let random_server_port = numbers.choose(&mut rng).unwrap();
    let random_server_port = rand::rng().random_range(50000..65000);
    let server_port: String = Input::new()
        .with_prompt("Server port: ")
        .default(random_server_port.to_string())
        .interact_text()
        .unwrap();
    let client_dns_1: String = Input::new()
        .with_prompt("DNS 1: ")
        .default("1.1.1.1".to_string())
        .interact_text()
        .unwrap();
    let client_dns_2: String = Input::new()
        .with_prompt("DNS 2: ")
        .default("1.0.0.1".to_string())
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
    let answers = InstallAnswers {
        server_pub_ip: Ipv4Addr::from_str(server_public_ip.as_str())
            .expect("Failed to parse public IPv4 address"),
        server_public_nic,
        server_pub_ipv6: server_public_ipv6,
        server_wg_ip: Ipv4Addr::from_str(server_wg_ip.as_str()).expect("Failed to parse wg0 IP"),
        server_wg_nic: server_wg_interface,
        server_wg_port: server_port.parse::<u16>().expect("Failed to parse port"),
        client_dns_1: Ipv4Addr::from_str(client_dns_1.as_str()).expect("Failed to parse DNS 1"),
        client_dns_2: Ipv4Addr::from_str(client_dns_2.as_str()).expect("Failed to parse DNS 2"),
        allowed_ips,
    };
    println!(
        r#"
    Okay, that was all I needed. We are ready to setup your WireGuard server now.
    You will be able to generate a client at the end of the installation.
    Press enter to contiune
    "#
    );
    io::stdin().read_line(&mut String::new()).unwrap();
    answers
}
pub fn get_home_dir_for_client(client_name: &String) -> PathBuf {
    let path = PathBuf::from("/home/").join(client_name);
    let exists: bool = path.exists();
    let is_dir: bool = path.is_dir();
    let home_dir: PathBuf = if exists && is_dir {
        path
    } else {
        PathBuf::from_str("/etc/wireguard/clients.d/")
            .expect("Failed to acces /etc/wireguard/clients.d")
    };
    home_dir
}
pub async fn initial_check() -> io::Result<()> {
    let _ = check_virtualization().await;
    let _ = is_root();
    let _ = check_os();
    Ok(())
}
pub fn check_os() -> io::Result<()> {
    let os = get_os();
    match os {
        OsType::Debian | OsType::Rasbian => {
            let version = std::env::var("VERSION_ID").expect("Failed to get version ID");
            if version
                .parse::<u8>()
                .expect("Failed to parse Debian version number")
                < 10
            {
                eprintln!("Please use Debian 10 Buster or later");
                std::process::exit(1);
            }
        }
        OsType::Ubuntu => {
            let version_id = std::env::var("VERSION_ID").expect("Failed to get version ID");
            let release_year = version_id
                .split(".")
                .next()
                .expect("Failed to get major version id")
                .parse::<u8>()
                .expect("Failed to parse version ID");
            if release_year < 18 {
                eprintln!("Please use Ubuntu 18 or later");
                std::process::exit(1)
            }
        }
        OsType::Fedora => {
            if std::env::var("VERSION_ID")
                .expect("Failed to get version ID")
                .parse::<u8>()
                .expect("Failed to parse Fedora version ID")
                < 32
            {
                eprintln!("Please use Fedora 32 or later");
                std::process::exit(1)
            }
        }
        OsType::Centos | OsType::AlmaLinux | OsType::Rocky => {
            if std::env::var("VERSION_ID")
                .expect("Failed to get version ID")
                .parse::<u8>()
                .expect("Failed to parse version ID")
                < 7
            {
                eprintln!("Please use CentOS 8 or later");
                std::process::exit(1)
            }
        }
        OsType::Arch => print!(""),
        OsType::Alpine => print!(""),
        OsType::Unknown => {
            eprintln!(
                "Looks like you aren't running this installer on a Debian, Ubuntu, Fedora, CentOS, AlmaLinux or Arch Linux system"
            );
            std::process::exit(2);
        }
    }
    Ok(())
}
pub fn get_os() -> OsType {
    dotenv::from_path("/etc/os-release")
        .expect("Failed to load /etc/os-release environment variable");
    let os = match std::env::var("NAME") {
        Ok(os) => os.to_lowercase(),
        Err(e) => {
            eprintln!("Something went wrong getting OS information, please check supported OSes.");
            eprintln!("If your os is supported one, please report it.");
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    match os.as_str() {
        "debian" | "rasbian" => OsType::Debian,
        "ubuntu" => OsType::Ubuntu,
        "fedora" => OsType::Fedora,
        "centos" => OsType::Centos,
        "almalinux" => OsType::AlmaLinux,
        "rocky" => OsType::Rocky,
        "arch" => OsType::Arch,
        _ => OsType::Unknown,
    }
}
pub async fn check_virtualization() -> io::Result<()> {
    let virtualiation = heim_virt::detect()
        .await
        .expect("Failed to detect virtualization");
    if virtualiation == heim_virt::Virtualization::Lxc {
        eprintln!(
            r#"
        LXC is not supported (yet).
        WireGuard can technically run in an LXC container,
        but the kernel module has to be installed on the host,
        the container has to be run with some specific parameters
        and only the tools need to be installed in the container.
        "#
        );
        std::process::exit(1);
    } else if virtualiation == heim_virt::Virtualization::OpenVz {
        eprintln!("OpenVZ is not supported");
        std::process::exit(1);
    }
    Ok(())
}

pub fn is_root() -> io::Result<()> {
    if unsafe { libc::getuid() } != 0 {
        eprintln!("You must be root to run in a container");
        std::process::exit(1);
    };
    Ok(())
}
