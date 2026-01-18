use std::cmp::min;
use tokio;

#[tokio::main]
async fn main() {
    check_virtualization().await;
    is_root();
    check_os();
}

pub fn check_os(){
    let os: String = get_os();
    dotenv::from_path("/etc/os-release").unwrap();
    match os.as_str() {
        "debian" | "rasbian" => {
            let version = std::env::var("VERSION_ID").unwrap();
            if version.parse::<u8>().expect("Failed to parse Debian version number") < 10{
                eprintln!("Please use Debian 10 Buster or later");
                std::process::exit(1);
            }
        },
        "ubuntu" => {
            let release_year = std::env::var("VERSION_ID").unwrap().split_once(".").unwrap().1.parse::<u8>().unwrap();
            if release_year < 18{
                eprintln!("Please use Ubuntu 18 or later");
                std::process::exit(1)
            }
        },
        "fedora" => {
           if std::env::var("VERSION_ID").unwrap().parse::<u8>().unwrap() < 32 {
               eprintln!("Please use Fedora 32 or later");
               std::process::exit(1)
           }
        },
        "centos" | "almalinux" | "rocky" => {
            if std::env::var("VERSION_ID").unwrap().parse::<u8>().unwrap() < 7 {
                eprintln!("Please use CentOS 8 or later");
                std::process::exit(1)
            }
        },
        _ => {
            eprintln!("Looks like you aren't running this installer on a Debian, Ubuntu, Fedora, CentOS, AlmaLinux, Oracle or Arch Linux system");
            std::process::exit(2);
        }
    }

}
pub fn get_os() -> String{
    dotenv::from_path("/etc/os-release").unwrap();
    let os = match std::env::var("OS"){
        Ok(os) => os,
        Err(e) => {
            eprintln!("Something went wrong getting OS information, please check supported OSes.");
            eprintln!("If your os is supported one, please report it.");
            std::process::exit(1);
        }
    };

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
