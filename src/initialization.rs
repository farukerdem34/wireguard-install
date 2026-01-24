use crate::checks::{check_virtualization, is_root};
use crate::install::install_wireguard;
use crate::client::{new_client, list_clients, revoke_client};
use crate::uninstall::uninstall_wireguard;
use crate::os_detection::get_os;
use dialoguer::Select;
use std::io;
use std::path::Path;

pub async fn initial_check() -> io::Result<()> {
    let _ = check_virtualization().await;
    let _ = is_root();
    let os = get_os();
    println!("Detected OS: {:?}", os);
    
    // Check if WireGuard is already installed by looking for params file
    if Path::new("/etc/wireguard/params").exists() {
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
            "New Client",
            "List Clients",
            "Revoke Client",
            "Uninstall Wireguard",
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
                // New Client
                if let Err(e) = new_client() {
                    println!("Error creating client: {}", e);
                }
            },
            1 => {
                // List Clients
                if let Err(e) = list_clients() {
                    println!("Error listing clients: {}", e);
                }
            },
            2 => {
                // Revoke Client
                if let Err(e) = revoke_client() {
                    println!("Error revoking client: {}", e);
                }
            },
            3 => {
                // Uninstall Wireguard
                if let Err(e) = uninstall_wireguard() {
                    println!("Error during uninstall: {}", e);
                    return Err(e);
                }
                // Uninstall completes successfully, exit the program
                break;
            },
            4 => {
                // Exit
                println!("👋 Goodbye!");
                break;
            },
            _ => {
                println!("Invalid selection");
            }
        }
    }
    
    Ok(())
}

