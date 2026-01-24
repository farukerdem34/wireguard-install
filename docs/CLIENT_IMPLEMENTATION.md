# WireGuard Client Management - Rust Implementation

This document describes the complete Rust implementation of WireGuard client creation functionality, replacing the original bash script with a robust, type-safe solution.

## Overview

The `src/client.rs` module provides comprehensive WireGuard client management functionality that:

- ✅ **Loads server configuration** from `/etc/wireguard/params`
- ✅ **Validates client names** with interactive prompts
- ✅ **Manages IP address allocation** (IPv4 and IPv6)
- ✅ **Generates cryptographic keys** (private, public, preshared)
- ✅ **Creates client configuration files** with proper formatting
- ✅ **Updates server configuration** with new client peers
- ✅ **Syncs live WireGuard configuration** without service restart
- ✅ **Generates QR codes** for mobile device setup

## Key Rust Features Utilized

### 1. Strong Type System
- `Ipv4Addr`/`Ipv6Addr` for compile-time IP address validation
- `PathBuf` for safe file path handling
- Custom structs (`WireguardParams`, `ClientConfig`) for structured data

### 2. Error Handling
- `Result<T, E>` for all fallible operations
- Detailed error messages with context
- Proper error propagation using `?` operator

### 3. Memory Safety
- Automatic string memory management
- No buffer overflows or memory leaks
- Safe process execution with proper cleanup

### 4. Pattern Matching
- Regex-based client name validation
- Shell variable parsing with pattern recognition
- IPv6 bracket formatting logic

## Core Structures

### WireguardParams
```rust
pub struct WireguardParams {
    pub server_pub_nic: String,      // Public network interface
    pub server_wg_nic: String,       // WireGuard interface name
    pub server_wg_ipv4: Ipv4Addr,    // Server IPv4 address
    pub server_wg_ipv6: String,      // Server IPv6 address
    pub server_port: u16,            // Server port
    pub server_priv_key: String,     // Server private key
    pub server_pub_key: String,      // Server public key
    pub client_dns_1: Ipv4Addr,      // Primary DNS server
    pub client_dns_2: Ipv4Addr,      // Secondary DNS server
    pub allowed_ips: String,         // Allowed IP ranges
}
```

### ClientConfig
```rust
pub struct ClientConfig {
    pub name: String,                // Client name
    pub private_key: String,         // Client private key
    pub public_key: String,          // Client public key
    pub preshared_key: String,       // Preshared key
    pub ipv4: Ipv4Addr,             // Client IPv4 address
    pub ipv6: String,               // Client IPv6 address
    pub home_dir: PathBuf,          // Client home directory
}
```

## Main Functions

### new_client() -> Result<(), String>
Main entry point for interactive client creation. This function:
1. Loads server parameters from `/etc/wireguard/params`
2. Prompts for and validates client name
3. Allocates available IP addresses
4. Generates cryptographic keys
5. Creates client configuration file
6. Updates server configuration
7. Syncs live WireGuard configuration
8. Generates QR code for mobile setup

### load_wireguard_params() -> Result<WireguardParams, String>
Loads and parses server configuration from `/etc/wireguard/params`. Features:
- **Shell variable format parsing** (`KEY=VALUE` and `KEY=${VAR}`)
- **Type validation** for IP addresses and port numbers
- **Comprehensive error handling** for missing/invalid files
- **Clear error messages** for troubleshooting

## File Format Support

The implementation supports the shell variable format used by WireGuard installation scripts:

```bash
SERVER_PUB_NIC=${SERVER_PUB_NIC}
SERVER_WG_NIC=${SERVER_WG_NIC}
SERVER_WG_IPV4=${SERVER_WG_IPV4}
SERVER_WG_IPV6=${SERVER_WG_IPV6}
SERVER_PORT=${SERVER_PORT}
SERVER_PRIV_KEY=${SERVER_PRIV_KEY}
SERVER_PUB_KEY=${SERVER_PUB_KEY}
CLIENT_DNS_1=${CLIENT_DNS_1}
CLIENT_DNS_2=${CLIENT_DNS_2}
ALLOWED_IPS=${ALLOWED_IPS}
```

## Key Generation Process

The implementation uses native WireGuard tools for cryptographic operations:

1. **Private Key**: `wg genkey`
2. **Public Key**: `echo "<private_key>" | wg pubkey`
3. **Preshared Key**: `wg genpsk`

All key generation is handled securely with proper error checking and UTF-8 validation.

## IP Address Management

### IPv4 Allocation
- Extracts base network from server IPv4 (`192.168.1.0/24` → `192.168.1`)
- Tests addresses from `x.x.x.2` to `x.x.x.254`
- Checks against existing server configuration for conflicts
- Supports up to 253 clients per subnet

### IPv6 Allocation
- Supports standard IPv6 subnet notation (`2001:db8::/64`)
- Allocates sequential addresses in the subnet
- Validates against existing client configurations

## Configuration File Generation

### Client Configuration Format
```ini
[Interface]
PrivateKey = <client_private_key>
Address = <client_ipv4>/32,<client_ipv6>/128
DNS = <dns1>,<dns2>

# MTU configuration comments included

[Peer]
PublicKey = <server_public_key>
PresharedKey = <preshared_key>
Endpoint = <server_ip>:<port>
AllowedIPs = <allowed_ranges>
```

### Server Configuration Update
Appends client as peer to server configuration:
```ini
### Client <client_name>
[Peer]
PublicKey = <client_public_key>
PresharedKey = <preshared_key>
AllowedIPs = <client_ipv4>/32,<client_ipv6>/128
```

## QR Code Generation

Uses the `qrcode` crate to generate Unicode-based QR codes displayed in terminal:
- **High density rendering** using Unicode block characters
- **Mobile-friendly format** for easy scanning
- **Error handling** for QR generation failures

## Usage Example

```rust
use crate::client;

// Interactive client creation
match client::new_client() {
    Ok(()) => println!("✅ Client created successfully!"),
    Err(e) => println!("❌ Error: {}", e),
}

// Load server parameters
match client::load_wireguard_params() {
    Ok(params) => {
        println!("Server: {}:{}", params.server_wg_ipv4, params.server_port);
    },
    Err(e) => println!("❌ Configuration error: {}", e),
}
```

## Dependencies

The implementation requires these additional dependencies:
- `regex = "1.10.0"` - Client name validation
- `qrcode = "0.14.1"` - QR code generation

Existing dependencies are reused:
- `dialoguer` - Interactive CLI prompts
- `std::process::Command` - WireGuard tool execution

## Advantages Over Bash Script

### 1. **Memory Safety**
- No buffer overflows or string manipulation errors
- Automatic memory management
- Safe concurrent operations

### 2. **Type Safety**
- Compile-time validation of IP addresses
- Strong typing prevents configuration errors
- Structured error handling

### 3. **Error Handling**
- Detailed error messages with context
- Graceful handling of missing files/permissions
- Clear troubleshooting guidance

### 4. **Performance**
- Faster execution than shell scripts
- Efficient regex compilation
- Optimized file I/O operations

### 5. **Maintainability**
- Self-documenting code with type signatures
- Modular function design
- Comprehensive error reporting

### 6. **Cross-platform Compatibility**
- Works on any system with Rust and WireGuard tools
- Consistent behavior across platforms
- No shell-specific dependencies

## Integration

The module is designed as a standalone library that can be integrated into the existing WireGuard installation tool or used independently:

```rust
// In main.rs
mod client;

// Usage in CLI menu
match user_choice {
    "create_client" => client::new_client()?,
    // ... other options
}
```

## Testing

The implementation can be tested with:
```bash
cargo check    # Compile-time validation
cargo build    # Full build
cargo test     # Unit tests (when added)
```

## Future Enhancements

Potential improvements include:
- Unit tests for individual functions
- Client removal functionality
- Client listing and management
- Configuration validation utilities
- Backup and restore capabilities

---

This implementation provides a robust, type-safe replacement for bash-based WireGuard client management while maintaining full compatibility with existing WireGuard configurations and workflows.