use crate::enums::OsType;
use dotenv;

pub fn get_os() -> OsType {
    dotenv::from_path("/etc/os-release")
        .expect("Failed to load /etc/os-release environment variable");
    let os = match std::env::var("NAME") {
        Ok(os) => os.to_lowercase(),
        Err(e) => {
            eprintln!("Something went wrong getting OS information. Please check supported OSes.");
            eprintln!("If your OS is supported, please report it.");
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
