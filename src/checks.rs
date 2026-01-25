use heim_virt;
use anyhow::{Result, Context, bail};

pub async fn check_virtualization() -> Result<()> {
    let virtualization = heim_virt::detect()
        .await
        .context("Failed to detect virtualization")?;
    if virtualization == heim_virt::Virtualization::Lxc {
        bail!("LXC is not supported. Refer to documentation for details.");
    } else if virtualization == heim_virt::Virtualization::OpenVz {
        bail!("OpenVZ is not supported.");
    }
    Ok(())
}

pub fn is_root() -> Result<()> {
    if unsafe { libc::getuid() } != 0 {
        bail!("You must be root to run this installer.");
    };
    Ok(())
}
