use std::io;
use dialoguer::Input;
use rand::prelude::IndexedRandom;
use tokio;
pub struct InstallAnswers {
    server_pub_ip: String,
    server_wg_nic: String,
    server_wg_ip: String,
    server_wg_port: u16,
    client_dns_1: String,
    client_dns_2: String,
    allowed_ips: String,
}
#[tokio::main]
async fn main() {
    initialCheck().await;
    let install_answers: InstallAnswers = install_question();
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
        let interfaces = netwatcher::list_interfaces().unwrap();
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
        .default(predicted_server_public_ip.unwrap().into())
        .interact_text()
        .unwrap();
    let server_public_nic: String = Input::new()
        .with_prompt("Public interface: ")
        .default(predicted_server_public_nic.unwrap().into())
        .interact_text()
        .unwrap();
    let server_wg_interface: String = Input::new()
        .with_prompt("WireGuard interface name: ")
        .default("wg0".to_string().into())
        .interact_text()
        .unwrap();
    let server_wg_ip: String = Input::new()
        .with_prompt("Server WireGuard IPv4: ")
        .default("10.19.11.1".to_string().into())
        .interact_text()
        .unwrap();
    let mut rng = rand::rng();
    let numbers: Vec<u16> = (50000..65000).collect();
    let random_server_port = numbers.choose(&mut rng).unwrap();
    let server_port: String = Input::new()
        .with_prompt("Server port: ")
        .default(random_server_port.to_string().into())
        .interact_text()
        .unwrap();
    let client_dns_1: String = Input::new()
        .with_prompt("DNS 1: ")
        .default("1.1.1.1".to_string().into())
        .interact_text()
        .unwrap();
    let client_dns_2: String = Input::new()
        .with_prompt("DNS 2: ")
        .default("1.0.0.1".to_string().into())
        .interact_text()
        .unwrap();
    let allowed_ips: String = Input::new()
        .with_prompt(
            r#"
        WireGuard uses a parameter called AllowedIPs to determine what is routed over the VPN.
        Allowed IPs list for generated clients (leave default to route everything):
        "#,
        )
        .default("0.0.0.0/0".to_string().into())
        .interact_text()
        .unwrap();
    let answers = InstallAnswers {
        server_pub_ip: server_public_ip,
        server_wg_ip: server_wg_ip,
        server_wg_nic: server_wg_interface,
        server_wg_port: server_port.parse::<u16>().unwrap(),
        client_dns_1: client_dns_1,
        client_dns_2: client_dns_2,
        allowed_ips: allowed_ips,
    };
    println!(r#"
    Okay, that was all I needed. We are ready to setup your WireGuard server now.
    You will be able to generate a client at the end of the installation.
    Press enter to contiune
    "#);
    io::stdin().read_line(&mut String::new()).unwrap();
    answers
}
pub fn get_home_dir_for_client(client_name: &String) -> String {
    let mut home_dir: String = String::new();

    let path_string = format!("/home/{}", &client_name);
    let path = std::path::Path::new(&path_string);
    let exists: bool = path.exists();
    let is_dir: bool = path.is_dir();
    if exists && is_dir {
        home_dir = path_string;
    } else {
        home_dir = "/opt/wireguard-clients.d".to_string();
    }
    home_dir
}
pub async fn initialCheck() {
    check_virtualization().await;
    is_root();
    check_os();
}
pub fn check_os() {
    let os: String = get_os();
    dotenv::from_path("/etc/os-release").unwrap();
    match os.as_str() {
        "debian" | "rasbian" => {
            let version = std::env::var("VERSION_ID").unwrap();
            if version
                .parse::<u8>()
                .expect("Failed to parse Debian version number")
                < 10
            {
                eprintln!("Please use Debian 10 Buster or later");
                std::process::exit(1);
            }
        }
        "ubuntu" => {
            let release_year = std::env::var("VERSION_ID")
                .unwrap()
                .split_once(".")
                .unwrap()
                .1
                .parse::<u8>()
                .unwrap();
            if release_year < 18 {
                eprintln!("Please use Ubuntu 18 or later");
                std::process::exit(1)
            }
        }
        "fedora" => {
            if std::env::var("VERSION_ID").unwrap().parse::<u8>().unwrap() < 32 {
                eprintln!("Please use Fedora 32 or later");
                std::process::exit(1)
            }
        }
        "centos" | "almalinux" | "rocky" => {
            if std::env::var("VERSION_ID").unwrap().parse::<u8>().unwrap() < 7 {
                eprintln!("Please use CentOS 8 or later");
                std::process::exit(1)
            }
        },
        "arch" => print!(""),
        _ => {
            eprintln!(
                "Looks like you aren't running this installer on a Debian, Ubuntu, Fedora, CentOS, AlmaLinux, Oracle or Arch Linux system"
            );
            std::process::exit(2);
        }
    }
}
pub fn get_os() -> String {
    dotenv::from_path("/etc/os-release").unwrap();
    let os = match std::env::var("NAME") {
        Ok(os) => os.to_lowercase(),
        Err(e) => {
            eprintln!("Something went wrong getting OS information, please check supported OSes.");
            eprintln!("If your os is supported one, please report it.");
            std::process::exit(1);
        }
    };
    unsafe {
        std::env::set_var("OS", &os);
    }
    os
}
pub async fn check_virtualization() {
    let virtualiation = heim_virt::detect().await.unwrap();
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
}

pub fn is_root() {
    if unsafe { libc::getuid() } != 0 {
        eprintln!("You must be root to run in a container");
        std::process::exit(1);
    }
}
