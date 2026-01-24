use crate::checks::{check_virtualization, is_root};
use crate::install::install_wireguard;
use crate::client::{new_client, list_clients, revoke_client};
use crate::uninstall::uninstall_wireguard;
use crate::os_detection::get_os;
use dialoguer::Select;
use std::io;

pub async fn initial_check() -> io::Result<()> {
    let _ = check_virtualization().await;
    let _ = is_root();
    let os = get_os();
    println!("Detected OS: {:?}", os);
    
    // Show main menu
    if let Err(e) = show_main_menu(os) {
        eprintln!("Error: {}", e);
    }
    
    Ok(())
}

fn show_main_menu(os: crate::enums::OsType) -> Result<(), String> {
    loop {
        println!();
        println!("=== WireGuard Management ===");
        println!();
        
        let options = vec![
            "Install WireGuard",
            "Manage Clients",
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
                // Install WireGuard
                println!();
                println!("Starting WireGuard installation...");
                install_wireguard(os);
                break;
            },
            1 => {
                // Manage Clients
                if let Err(e) = show_client_menu() {
                    println!("Error: {}", e);
                }
            },
            2 => {
                // Uninstall WireGuard
                if let Err(e) = uninstall_wireguard() {
                    println!("Error during uninstall: {}", e);
                    return Err(e);
                }
                break;
            },
            3 => {
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

fn show_client_menu() -> Result<(), String> {
    loop {
        println!();
        println!("=== Client Management ===");
        println!();
        
        let options = vec![
            "Add new client",
            "List existing clients", 
            "Revoke client",
            "Back to main menu",
        ];
        
        let selection = Select::new()
            .with_prompt("Client management options:")
            .items(&options)
            .default(0)
            .interact()
            .map_err(|e| format!("Client menu selection error: {}", e))?;
        
        match selection {
            0 => {
                // Add new client
                if let Err(e) = new_client() {
                    println!("Error creating client: {}", e);
                }
            },
            1 => {
                // List existing clients
                if let Err(e) = list_clients() {
                    println!("Error listing clients: {}", e);
                }
            },
            2 => {
                // Revoke client
                if let Err(e) = revoke_client() {
                    println!("Error revoking client: {}", e);
                }
            },
            3 => {
                // Back to main menu
                break;
            },
            _ => {
                println!("Invalid selection");
            }
        }
    }
    
    Ok(())
}