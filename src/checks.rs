use heim_virt;
use std::io;

pub async fn check_virtualization() -> io::Result<()> {
    let virtualization = heim_virt::detect()
        .await
        .expect("Failed to detect virtualization");
    if virtualization == heim_virt::Virtualization::Lxc {
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
    } else if virtualization == heim_virt::Virtualization::OpenVz {
        eprintln!("OpenVZ is not supported");
        std::process::exit(1);
    }
    Ok(())
}

pub fn is_root() -> io::Result<()> {
    if unsafe { libc::getuid() } != 0 {
        eprintln!("You must be root to run this installer.");
        std::process::exit(1);
    };
    Ok(())
}
