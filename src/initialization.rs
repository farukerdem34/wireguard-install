use crate::checks::{check_virtualization, is_root};
use crate::os_detection::get_os;
use std::io;

pub async fn initial_check() -> io::Result<()> {
    let _ = check_virtualization().await;
    let _ = is_root();
    let os = get_os();
    println!("Detected OS: {:?}", os);
    Ok(())
}