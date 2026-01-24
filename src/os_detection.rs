use crate::enums::OsType;
use crate::models::VersionInfo;
use dotenv;

pub fn get_os() -> OsType {
    let (os_type, _) = get_os_with_version();
    os_type
}

pub fn get_os_with_version() -> (OsType, VersionInfo) {
    dotenv::from_path("/etc/os-release")
        .expect("Failed to load /etc/os-release environment variable");

    let os_name = match std::env::var("NAME") {
        Ok(os) => os.to_lowercase(),
        Err(e) => {
            eprintln!("Something went wrong getting OS information. Please check supported OSes.");
            eprintln!("If your OS is supported, please report it.");
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let version_id = std::env::var("VERSION_ID").unwrap_or_default();
    let version_info = VersionInfo::new(&version_id);

    let os_type = match os_name.as_str() {
        name if name.contains("debian") => OsType::Debian,
        name if name.contains("raspbian") => OsType::Raspbian,
        name if name.contains("ubuntu") => OsType::Ubuntu,
        name if name.contains("fedora") => OsType::Fedora,
        name if name.contains("centos") => OsType::Centos,
        name if name.contains("almalinux") => OsType::AlmaLinux,
        name if name.contains("rocky") => OsType::Rocky,
        name if name.contains("oracle") => OsType::Oracle,
        name if name.contains("arch") => OsType::Arch,
        name if name.contains("alpine") => OsType::Alpine,
        _ => OsType::Unknown,
    };

    (os_type, version_info)
}
