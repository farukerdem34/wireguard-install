# WireGuard VPN Server Installer

Original project: [angristan/wireguard-install](https://github.com/angristan/wireguard-install.git)

A zero-configuration WireGuard VPN server installer and management tool that works across multiple Linux distributions. Set up a secure VPN server in minutes with automated installation, client management, and QR code generation.

## ✨ Key Features

- **🌐 Cross-Platform Support**: Works on 8+ Linux distributions with automatic OS detection
- **⚡ Zero-Configuration Setup**: Automated installation with sensible defaults - no complex networking knowledge required
- **👥 User-Friendly Interface**: Interactive CLI with guided prompts and input validation
- **🔧 Complete Management**: Install, create clients, manage connections, and uninstall cleanly
- **📱 Mobile Ready**: Automatic QR code generation for easy mobile device setup
- **🔒 Security Focused**: Proper file permissions, firewall integration, and secure key generation
- **🔄 Full Lifecycle**: From installation to client management to complete removal

## 🚀 Quick Start

1. **Clone and build**:
   ```bash
   git clone https://github.com/your-username/wireguard-install.git
   cd wireguard-install
   cargo build --release
   ```

2. **Run as root**:
   ```bash
   sudo ./target/release/wireguard-install
   ```

3. **Follow the interactive prompts** - the installer will guide you through the entire setup process!

## 🖥️ Supported Operating Systems

| Distribution | Package Manager | Status |
|--------------|-----------------|---------|
| Debian/Ubuntu/Raspbian | `apt-get` | ✅ Supported |
| Fedora | `dnf` | ✅ Supported |
| CentOS/RHEL | `yum` | ✅ Supported |
| AlmaLinux | `dnf` | ✅ Supported |
| Rocky Linux | `dnf` | ✅ Supported |
| Oracle Linux | `yum` | ✅ Supported |
| Arch Linux | `pacman` | ✅ Supported |
| Alpine Linux | `apk` | ✅ Supported |

## 🛠️ What This Tool Does

The installer handles everything for you automatically:

### 1. **System Validation**
- Checks for root privileges
- Detects your operating system
- Verifies virtualization compatibility
- Validates network configuration

### 2. **Automated Installation**
- Updates your system's package repositories
- Installs WireGuard and required dependencies
- Installs additional tools (iptables, QR code utilities)
- Handles OS-specific package differences

### 3. **Smart Configuration**
- Auto-detects your public IP address
- Finds your primary network interface
- Generates secure server keys
- Creates optimized server configuration
- Sets up proper file permissions

### 4. **Network & Security Setup**
- Enables IP forwarding for VPN traffic
- Configures firewall rules (iptables or firewalld)
- Sets up NAT masquerading
- Opens necessary ports automatically

### 5. **Service Management**
- Enables WireGuard system service
- Starts the VPN server
- Configures automatic startup on boot

### 6. **First Client Setup**
- Creates your first VPN client automatically
- Generates client configuration file
- Displays QR code for mobile setup
- Saves config to your home directory

## 📋 Usage

### First Time Setup
When you run the tool for the first time, it will automatically start the installation wizard:

```bash
sudo ./target/release/wireguard-install
```

### Managing Existing Installation
If WireGuard is already installed, you'll see the management menu:

- **Add New Client**: Create additional VPN users with unique configurations
- **List Clients**: View all configured VPN clients and their details
- **Revoke Client**: Remove a client and update server configuration
- **Uninstall WireGuard**: Completely remove WireGuard and restore system state
- **Exit**: Close the management tool

### Client Management

#### Adding New Clients
1. Select "Add New Client" from the menu
2. Enter a name for the client
3. Choose IP addressing (IPv4/IPv6 options)
4. Configure DNS settings
5. Set traffic routing (all traffic or specific networks)
6. Get instant QR code for mobile devices

#### Using Client Configurations
- **Desktop/Laptop**: Import the `.conf` file into your WireGuard client
- **Mobile Devices**: Scan the QR code with the WireGuard mobile app
- **Configuration files** are saved to your home directory as `client-name.conf`

## 📋 Requirements

### System Requirements
- Linux operating system with systemd
- Root or sudo access
- One of the supported distributions (see table above)
- Internet connection for package installation
- Compatible virtualization environment (LXC/OpenVZ are not supported)

### Network Requirements
- Public IP address (automatically detected)
- Available UDP port (default: 51820, customizable)
- Network interface with internet access
- Firewall that allows configuration changes

## 🔧 Building from Source

### Prerequisites
- Rust toolchain (rustc, cargo)
- Git

### Build Instructions
```bash
# Clone the repository
git clone https://github.com/your-username/wireguard-install.git
cd wireguard-install

# Build the project
cargo build --release

# The executable will be available at:
./target/release/wireguard-install
```

### Development Build
```bash
# For development/testing
cargo build

# Run directly with cargo
cargo run
```

## 🔒 Security Features

- **Secure Key Generation**: Uses WireGuard's native cryptographic key generation
- **Proper Permissions**: Sets restrictive file permissions (700/600) on all configuration files
- **Firewall Integration**: Automatically configures firewall rules for your system
- **Input Validation**: Validates all user inputs to prevent configuration errors
- **IP Conflict Prevention**: Automatically prevents IP address conflicts between clients
- **Clean Uninstallation**: Completely removes all traces and restores original system state

## 🤔 How It Works

1. **Detection Phase**: The tool detects your OS, network setup, and system capabilities
2. **Installation Phase**: Packages are installed using your distribution's package manager
3. **Configuration Phase**: Server keys are generated and configuration files are created
4. **Network Phase**: Firewall rules and IP forwarding are configured
5. **Service Phase**: WireGuard service is enabled and started
6. **Client Phase**: Your first VPN client is created with QR code
7. **Management Phase**: Ongoing client management through the interactive menu

## 📁 Generated Files

The installer creates these files during setup:
- `/etc/wireguard/wg0.conf` - Main server configuration
- `/etc/wireguard/params` - Installation parameters and settings
- `~/client-name.conf` - Individual client configuration files

## 🚫 Uninstalling

To completely remove WireGuard and restore your system:

1. Run the tool: `sudo ./target/release/wireguard-install`
2. Select "Uninstall WireGuard" from the menu
3. Confirm the uninstallation
4. The tool will:
   - Stop the WireGuard service
   - Remove all configuration files
   - Uninstall WireGuard packages
   - Remove firewall rules
   - Disable IP forwarding
   - Clean up all generated files

## 🤝 Contributing

Contributions are welcome! Please feel free to:
- Report bugs or issues
- Suggest new features
- Submit pull requests
- Improve documentation

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⭐ Why Use This Tool?

- **Save Time**: No need to manually configure complex networking, firewall rules, or service management
- **Reduce Errors**: Automated setup prevents common configuration mistakes
- **Cross-Platform**: Works consistently across different Linux distributions
- **Production Ready**: Includes proper security measures and best practices
- **Beginner Friendly**: No advanced networking knowledge required
- **Complete Solution**: Handles everything from installation to ongoing management
- **Mobile Ready**: QR codes make mobile device setup effortless

Get your secure VPN server running in minutes, not hours!