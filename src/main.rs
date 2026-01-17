use tokio;

#[tokio::main]
async fn main() {
    check_virtualization().await;
}

pub async fn check_virtualization(){
    let virtualiation = heim_virt::detect().await.unwrap();
    if virtualiation == heim_virt::Virtualization::Lxc {
        eprintln!(r#"
        LXC is not supported (yet).
        WireGuard can technically run in an LXC container,
        but the kernel module has to be installed on the host,
        the container has to be run with some specific parameters
        and only the tools need to be installed in the container.
        "#);
        std::process::exit(1);
    } else if virtualiation == heim_virt::Virtualization::OpenVz {
        eprintln!("OpenVZ is not supported");
        std::process::exit(1);
    }
}
