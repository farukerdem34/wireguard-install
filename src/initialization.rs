use crate::checks::{check_virtualization, is_root};
use crate::client::{list_clients, new_client, revoke_client};
use crate::install::install_wireguard;
use crate::interface::{create_new_interface, list_interfaces, remove_interface};
use crate::migration::check_migration_status;
use crate::os_detection::get_os;
use crate::uninstall::uninstall_wireguard;
use crate::utils::clear_terminal;
use dialoguer::Select;
use std::io;
use std::path::Path;

pub async fn initial_check() -> io::Result<()> {
    let _ = check_virtualization().await;
    let _ = is_root();
    let os = get_os();
    println!("Detected OS: {:?}", os);

    // Check for migration needs and perform if necessary
    if let Err(e) = check_migration_status() {
        eprintln!("Migration error: {}", e);
        return Ok(());
    }

    // Check if WireGuard is installed (either old format or new format)
    if Path::new("/etc/wireguard/params").exists() || Path::new("/etc/wireguard/interfaces.json").exists() {
        // WireGuard is installed, show management menu
        if let Err(e) = show_management_menu() {
            eprintln!("Error: {}", e);
        }
    } else {
        // WireGuard is not installed, run installation
        println!("WireGuard not detected. Starting installation...");
        install_wireguard(os);
    }

    Ok(())
}

fn show_management_menu() -> Result<(), String> {
    loop {
        println!();
        println!("=== WireGuard Management ===");
        println!();

        let options = vec![
            "Add Client",
            "List Clients", 
            "Revoke Client",
            "Add Interface",
            "List Interfaces",
            "Remove Interface",
            "Uninstall WireGuard",
            "Exit",
        ];

        let selection = Select::new()
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact()
            .map_err(|e| format!("Menu selection error: {}", e))?;

        match selection {
            0 => {
                // Add Client
                if let Err(e) = new_client() {
                    println!("Error creating client: {}", e);
                }
            }
            1 => {
                // List Clients
                if let Err(e) = list_clients() {
                    println!("Error listing clients: {}", e);
                }
                clear_terminal();
            }
            2 => {
                // Revoke Client
                if let Err(e) = revoke_client() {
                    println!("Error revoking client: {}", e);
                }
                clear_terminal();
            }
            3 => {
                // Add Interface
                if let Err(e) = create_new_interface() {
                    println!("Error creating interface: {}", e);
                }
                clear_terminal();
            }
            4 => {
                // List Interfaces
                if let Err(e) = list_interfaces() {
                    println!("Error listing interfaces: {}", e);
                }
                clear_terminal();
            }
            5 => {
                // Remove Interface
                if let Err(e) = remove_interface() {
                    println!("Error removing interface: {}", e);
                }
                clear_terminal();
            }
            6 => {
                // Uninstall WireGuard
                if let Err(e) = uninstall_wireguard() {
                    println!("Error during uninstall: {}", e);
                    return Err(e);
                }
                break;
            }
            7 => {
                // Exit
                println!("👋 Goodbye!");
                break;
            }
            _ => {
                println!("Invalid selection");
            }
        }
    }

    Ok(())
}
