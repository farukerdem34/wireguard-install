use crate::client::{WireguardParams, load_wireguard_params};
use crate::enums::OsType;
use crate::models::VersionInfo;
use crate::os_detection::get_os_with_version;
use crate::utils::clear_terminal;
use dialoguer::{Confirm, Select};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Main entry point for WireGuard uninstallation
pub fn uninstall_wireguard() -> Result<(), String> {
    println!("🔍 Checking WireGuard installation...");

    // Step 1: Check if WireGuard is installed
    let wg_config = match discover_wireguard_installation() {
        Ok(config) => config,
        Err(e) => {
            println!("❌ WireGuard installation not found: {}", e);
            return Ok(());
        }
    };

    println!(
        "✓ WireGuard installation detected (interface: {})",
        wg_config.server_wg_nic
    );

    // Step 2: Show warning and get user confirmation
    if !confirm_uninstall()? {
        println!("🚫 Uninstall cancelled by user");
        clear_terminal();
        return Ok(());
    }

    // Step 3: Ask about backup
    let backup_requested = ask_for_backup()?;
    if backup_requested {
        backup_configuration(&wg_config)?;
    }

    // Clear terminal after user interactions
    clear_terminal();

    println!("🔧 Starting WireGuard uninstallation...");

    // Get OS information
    let (os_type, version_info) = get_os_with_version();
    println!(
        "📋 Detected OS: {:?} (version: {})",
        os_type, version_info.full_version
    );

    // Step 4: Stop and disable WireGuard service
    stop_wireguard_service(&wg_config.server_wg_nic, &os_type)?;
    disable_wireguard_service(&wg_config.server_wg_nic, &os_type)?;

    // Step 5: Remove packages
    remove_packages(&os_type, &version_info)?;

    // Step 6: Clean up configuration files
    cleanup_configuration_files(&os_type)?;

    // Step 7: Verify uninstallation
    verify_uninstall(&wg_config.server_wg_nic, &os_type)?;

    println!("✅ WireGuard uninstalled successfully!");
    Ok(())
}

/// Discover WireGuard installation by checking configuration
fn discover_wireguard_installation() -> Result<WireguardParams, String> {
    // Try to load WireGuard parameters
    load_wireguard_params()
        .map_err(|e| format!("WireGuard configuration not found or not accessible: {}", e))
}

/// Display warning and get user confirmation for uninstall
fn confirm_uninstall() -> Result<bool, String> {
    println!();
    println!(
        "\x1b[31m⚠️  WARNING: This will uninstall WireGuard and remove all configuration files!\x1b[0m"
    );
    println!(
        "\x1b[33m📁 Please backup the /etc/wireguard directory if you want to keep your configuration files.\x1b[0m"
    );
    println!();

    Confirm::new()
        .with_prompt("Do you really want to remove WireGuard?")
        .default(false)
        .interact()
        .map_err(|e| format!("Failed to get user confirmation: {}", e))
}

/// Ask user if they want to backup client configurations
fn ask_for_backup() -> Result<bool, String> {
    println!();
    let options = vec![
        "Yes, create backup before removal",
        "No, proceed without backup",
    ];

    let selection = Select::new()
        .with_prompt("Would you like to backup your WireGuard configuration before removal?")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|e| format!("Failed to get backup preference: {}", e))?;

    Ok(selection == 0)
}

/// Create backup of WireGuard configuration
fn backup_configuration(_config: &WireguardParams) -> Result<(), String> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_dir = format!("/tmp/wireguard_backup_{}", timestamp);

    println!("📦 Creating backup at: {}", backup_dir);

    // Create backup directory
    fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;

    // Copy /etc/wireguard to backup location
    let output = Command::new("cp")
        .args(["-r", "/etc/wireguard", &backup_dir])
        .output()
        .map_err(|e| format!("Failed to execute backup command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Backup failed: {}", stderr));
    }

    println!("✓ Configuration backed up to: {}", backup_dir);
    Ok(())
}

/// Stop WireGuard service
fn stop_wireguard_service(interface: &str, os_type: &OsType) -> Result<(), String> {
    println!("⏹️  Stopping WireGuard service...");

    let output = match os_type {
        OsType::Alpine => Command::new("rc-service")
            .args([&format!("wg-quick.{}", interface), "stop"])
            .output(),
        _ => Command::new("systemctl")
            .args(["stop", &format!("wg-quick@{}", interface)])
            .output(),
    };

    let output = output.map_err(|e| format!("Failed to execute stop command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Don't fail if service is already stopped
        if !stderr.contains("not loaded") && !stderr.contains("not found") {
            return Err(format!("Failed to stop WireGuard service: {}", stderr));
        }
    }

    println!("✓ WireGuard service stopped");
    Ok(())
}

/// Disable WireGuard service from auto-start
fn disable_wireguard_service(interface: &str, os_type: &OsType) -> Result<(), String> {
    println!("🚫 Disabling WireGuard service...");

    let output = match os_type {
        OsType::Alpine => {
            // Remove from default runlevel
            let del_output = Command::new("rc-update")
                .args(["del", &format!("wg-quick.{}", interface)])
                .output()
                .map_err(|e| format!("Failed to execute rc-update del: {}", e))?;

            // Remove init script
            let _ = Command::new("unlink")
                .args([&format!("/etc/init.d/wg-quick.{}", interface)])
                .output()
                .map_err(|e| format!("Failed to execute unlink: {}", e));

            // Remove sysctl from boot (if it was added)
            let _ = Command::new("rc-update").args(["del", "sysctl"]).output();

            del_output
        }
        _ => Command::new("systemctl")
            .args(["disable", &format!("wg-quick@{}", interface)])
            .output()
            .map_err(|e| format!("Failed to execute disable command: {}", e))?,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Don't fail if service is already disabled
        if !stderr.contains("not loaded") && !stderr.contains("not found") {
            return Err(format!("Failed to disable WireGuard service: {}", stderr));
        }
    }

    println!("✓ WireGuard service disabled");
    Ok(())
}

/// Remove WireGuard packages based on OS type
fn remove_packages(os_type: &OsType, version_info: &VersionInfo) -> Result<(), String> {
    println!("📦 Removing WireGuard packages...");

    let output = match os_type {
        OsType::Ubuntu | OsType::Debian | OsType::Raspbian => Command::new("apt-get")
            .args(["remove", "-y", "wireguard", "wireguard-tools", "qrencode"])
            .output(),
        OsType::Fedora => {
            // Base packages
            let base_result = Command::new("dnf")
                .args([
                    "remove",
                    "-y",
                    "--noautoremove",
                    "wireguard-tools",
                    "qrencode",
                ])
                .output()
                .map_err(|e| format!("Failed to remove base packages: {}", e))?;

            // Version-specific packages for older Fedora
            if version_info.major_version < 32 {
                let _ = Command::new("dnf")
                    .args(["remove", "-y", "--noautoremove", "wireguard-dkms"])
                    .output();

                let _ = Command::new("dnf")
                    .args(["copr", "disable", "-y", "jdoss/wireguard"])
                    .output();
            }

            Ok(base_result)
        }
        OsType::Centos | OsType::AlmaLinux | OsType::Rocky => {
            let base_result = Command::new("yum")
                .args(["remove", "-y", "--noautoremove", "wireguard-tools"])
                .output()
                .map_err(|e| format!("Failed to remove base packages: {}", e))?;

            // Version-specific packages for version 8.x
            if version_info.major_version == 8 {
                let _ = Command::new("yum")
                    .args([
                        "remove",
                        "-y",
                        "--noautoremove",
                        "kmod-wireguard",
                        "qrencode",
                    ])
                    .output();
            }

            Ok(base_result)
        }
        OsType::Oracle => Command::new("yum")
            .args([
                "remove",
                "-y",
                "--noautoremove",
                "wireguard-tools",
                "qrencode",
            ])
            .output(),
        OsType::Arch => Command::new("pacman")
            .args(["-Rs", "--noconfirm", "wireguard-tools", "qrencode"])
            .output(),
        OsType::Alpine => {
            // First try to uninstall custom qrencode build
            let qr_dir = "qrencode-4.1.1";
            if Path::new(qr_dir).exists() {
                let _ = Command::new("sh")
                    .args(["-c", &format!("cd {} && make uninstall", qr_dir)])
                    .output();

                let _ = fs::remove_dir_all(qr_dir);
            }

            // Remove qrencode build artifacts
            let _ = Command::new("sh")
                .args(["-c", "rm -rf qrencode-*"])
                .output();

            // Remove Alpine packages
            Command::new("apk")
                .args(["del", "wireguard-tools", "libqrencode", "libqrencode-tools"])
                .output()
        }
        _ => {
            return Err(format!("Unsupported OS type: {:?}", os_type));
        }
    };

    let output = output.map_err(|e| format!("Failed to execute package removal command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Don't fail if packages are already removed
        if !stderr.contains("not installed") && !stderr.contains("No match") {
            return Err(format!("Failed to remove packages: {}", stderr));
        }
    }

    println!("✓ WireGuard packages removed");
    Ok(())
}

/// Clean up WireGuard configuration files and directories
fn cleanup_configuration_files(os_type: &OsType) -> Result<(), String> {
    println!("🧹 Cleaning up configuration files...");

    // Remove /etc/wireguard directory
    if Path::new("/etc/wireguard").exists() {
        fs::remove_dir_all("/etc/wireguard")
            .map_err(|e| format!("Failed to remove /etc/wireguard: {}", e))?;
        println!("✓ Removed /etc/wireguard directory");
    }

    // Remove sysctl configuration
    if Path::new("/etc/sysctl.d/wg.conf").exists() {
        fs::remove_file("/etc/sysctl.d/wg.conf")
            .map_err(|e| format!("Failed to remove /etc/sysctl.d/wg.conf: {}", e))?;
        println!("✓ Removed /etc/sysctl.d/wg.conf");
    }

    // Reload sysctl to apply changes (except for Alpine)
    if *os_type != OsType::Alpine {
        let _ = Command::new("sysctl").args(["--system"]).output();
        println!("✓ Reloaded system configuration");
    }

    Ok(())
}

/// Verify that WireGuard has been successfully uninstalled
fn verify_uninstall(interface: &str, os_type: &OsType) -> Result<(), String> {
    println!("🔍 Verifying uninstallation...");

    // Check if service is still running
    let service_check = match os_type {
        OsType::Alpine => Command::new("rc-service")
            .args([&format!("wg-quick.{}", interface), "status"])
            .output(),
        _ => Command::new("systemctl")
            .args(["is-active", "--quiet", &format!("wg-quick@{}", interface)])
            .output(),
    };

    let service_output =
        service_check.map_err(|e| format!("Failed to check service status: {}", e))?;

    // If service check returns success (0), WireGuard is still running
    if service_output.status.success() {
        return Err("WireGuard service is still running - uninstall may have failed".to_string());
    }

    // Check if configuration directory still exists
    if Path::new("/etc/wireguard").exists() {
        return Err(
            "WireGuard configuration directory still exists - cleanup may have failed".to_string(),
        );
    }

    println!("✓ Verification complete - WireGuard successfully removed");
    Ok(())
}
