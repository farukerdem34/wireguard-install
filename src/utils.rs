use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn set_permissions_recursive(path: &Path) -> io::Result<()> {
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
