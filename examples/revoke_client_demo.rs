// Example demo of how the revoke_client function works

fn main() {
    println!("=== Demo: revoke_client Function ===\n");

    println!("This demonstrates the Rust equivalent of the bash revokeClient() function:\n");

    println!("Original Bash Function:");
    println!("```bash");
    println!("function revokeClient() {{");
    println!("    NUMBER_OF_CLIENTS=$(grep -c -E \"^### Client\" \"/etc/wireguard/${{SERVER_WG_NIC}}.conf\")");
    println!("    if [[ ${{NUMBER_OF_CLIENTS}} == '0' ]]; then");
    println!("        echo \"\"");
    println!("        echo \"You have no existing clients!\"");
    println!("        exit 1");
    println!("    fi");
    println!("");
    println!("    echo \"\"");
    println!("    echo \"Select the existing client you want to revoke\"");
    println!("    grep -E \"^### Client\" \"/etc/wireguard/${{SERVER_WG_NIC}}.conf\" | cut -d ' ' -f 3 | nl -s ') '");
    println!("    until [[ ${{CLIENT_NUMBER}} -ge 1 && ${{CLIENT_NUMBER}} -le ${{NUMBER_OF_CLIENTS}} ]]; do");
    println!("        if [[ ${{CLIENT_NUMBER}} == '1' ]]; then");
    println!("            read -rp \"Select one client [1]: \" CLIENT_NUMBER");
    println!("        else");
    println!(
        "            read -rp \"Select one client [1-${{NUMBER_OF_CLIENTS}}]: \" CLIENT_NUMBER"
    );
    println!("        fi");
    println!("    done");
    println!("");
    println!("    # match the selected number to a client name");
    println!("    CLIENT_NAME=$(grep -E \"^### Client\" \"/etc/wireguard/${{SERVER_WG_NIC}}.conf\" | cut -d ' ' -f 3 | sed -n \"${{CLIENT_NUMBER}}\"p)");
    println!("");
    println!("    # remove [Peer] block matching $CLIENT_NAME");
    println!("    sed -i \"/^### Client ${{CLIENT_NAME}}$/,/^$/d\" \"/etc/wireguard/${{SERVER_WG_NIC}}.conf\"");
    println!("");
    println!("    # remove generated client file");
    println!("    HOME_DIR=$(getHomeDirForClient \"${{CLIENT_NAME}}\")");
    println!("    rm -f \"${{HOME_DIR}}/${{SERVER_WG_NIC}}-client-${{CLIENT_NAME}}.conf\"");
    println!("");
    println!("    # restart wireguard to apply changes");
    println!("    wg syncconf \"${{SERVER_WG_NIC}}\" <(wg-quick strip \"${{SERVER_WG_NIC}}\")");
    println!("}}");
    println!("```\n");

    println!("Rust Function Features:");
    println!("✅ Interactive arrow-key selection (better than number input)");
    println!("✅ Safety confirmation prompt before revocation");
    println!("✅ Best effort cleanup (continues even if some steps fail)");
    println!("✅ Enhanced error handling with detailed diagnostics");
    println!("✅ Type-safe configuration parsing (safer than sed/grep)");
    println!("✅ Atomic file operations with proper error recovery");
    println!("✅ Comprehensive progress reporting");
    println!("✅ Cross-platform compatibility (no shell dependencies)\n");

    println!("Function Signature:");
    println!("```rust");
    println!("pub fn revoke_client() -> Result<(), String>");
    println!("```\n");

    println!("Usage Example:");
    println!("```rust");
    println!("use wireguard_install::client::revoke_client;");
    println!("");
    println!("match revoke_client() {{");
    println!("    Ok(()) => println!(\"Client revoked successfully\"),");
    println!("    Err(e) => eprintln!(\"Error: {{}}\", e),");
    println!("}}");
    println!("```\n");

    println!("Enhanced User Interface Flow:");
    println!("```");
    println!("🗑️  Revoke WireGuard Client");
    println!("");
    println!("Found 3 clients:");
    println!("  1) laptop-client");
    println!("  2) phone-client");
    println!("  3) work-desktop");
    println!("");
    println!("Select the client you want to revoke:");
    println!("> laptop-client");
    println!("  phone-client");
    println!("  work-desktop");
    println!("");
    println!("⚠️  WARNING: This action cannot be undone!");
    println!("   Client 'laptop-client' will lose VPN access immediately.");
    println!("? Are you sure you want to revoke 'laptop-client'? (y/N) y");
    println!("");
    println!("   ✓ Removed client configuration file: /home/user/wg0-client-laptop-client.conf");
    println!("");
    println!("✅ Client 'laptop-client' has been successfully revoked!");
    println!("   • Removed from server configuration");
    println!("   • Client configuration file deleted");
    println!("   • WireGuard service updated");
    println!("```\n");

    println!("Error Handling Example (Best Effort):");
    println!("```");
    println!("⚠️  Client 'phone-client' partially revoked:");
    println!("   ✓ Removed from server configuration");
    println!("   ✓ WireGuard service updated");
    println!("   ✗ Failed to remove client files: Permission denied");
    println!("");
    println!("Some operations failed, but the client may still lose access.");
    println!("Please check the errors above and resolve them manually if needed.");
    println!("```\n");

    println!("Key Improvements Over Bash Version:");
    println!("");
    println!("| Aspect | Bash Version | Rust Version |");
    println!("|--------|-------------|--------------|");
    println!("| **Selection UI** | Number input with validation loop | Arrow key selection with built-in validation |");
    println!("| **Safety** | Immediate revocation | Confirmation prompt with warning |");
    println!("| **Error Handling** | Fail fast, limited diagnostics | Best effort cleanup with detailed reporting |");
    println!("| **Config Parsing** | sed/grep (risky) | Smart line-by-line parsing (safe) |");
    println!("| **File Operations** | rm -f (basic) | Atomic operations with existence checks |");
    println!("| **User Feedback** | Minimal output | Enhanced progress reporting |");
    println!("| **Type Safety** | Shell strings | Structured data with validation |");
    println!("| **Performance** | Multiple external commands | Single-pass in-memory processing |");
    println!("");

    println!("=== Implementation Complete ===");
    println!("The revoke_client function is ready for production use!");
}
