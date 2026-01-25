use std::fs;
use std::io;
use std::io::{stdin, stdout, Write};
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

/// Clear the terminal screen using ANSI escape codes
pub fn clear_terminal() {
    // Use ANSI escape codes to clear screen and move cursor to top
    print!("\x1B[2J\x1B[1;1H");
    stdout().flush().unwrap();
}

/// Wait for user to press any key to continue
pub fn wait_for_key_press() {
    print!("Press any key to continue...");
    stdout().flush().unwrap();

    // Read a line from stdin (user presses enter)
    let mut buffer = String::new();
    stdin().read_line(&mut buffer).unwrap();
}

/// Wait for user to press any key with custom message
pub fn wait_for_key_press_with_message(message: &str) {
    print!("{}", message);
    stdout().flush().unwrap();

    // Read a line from stdin (user presses enter)
    let mut buffer = String::new();
    stdin().read_line(&mut buffer).unwrap();
}
