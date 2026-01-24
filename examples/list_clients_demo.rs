// Example demo of how the list_clients function works

fn main() {
    println!("=== Demo: list_clients Function ===\n");

    println!("This demonstrates the Rust equivalent of the bash listClients() function:\n");

    println!("Original Bash Function:");
    println!("```bash");
    println!("function listClients() {{");
    println!("    NUMBER_OF_CLIENTS=$(grep -c -E \"^### Client\" \"/etc/wireguard/${{SERVER_WG_NIC}}.conf\")");
    println!("    if [[ ${{NUMBER_OF_CLIENTS}} -eq 0 ]]; then");
    println!("        echo \"\"");
    println!("        echo \"You have no existing clients!\"");
    println!("        exit 1");
    println!("    fi");
    println!("");
    println!("    grep -E \"^### Client\" \"/etc/wireguard/${{SERVER_WG_NIC}}.conf\" | cut -d ' ' -f 3 | nl -s ') '");
    println!("}}");
    println!("```\n");

    println!("Rust Function Features:");
    println!("✅ Loads environment variables from /etc/wireguard/params");
    println!("✅ Enhanced error handling with descriptive messages");
    println!("✅ Type-safe configuration parsing");
    println!("✅ Single-pass processing (no external commands)");
    println!("✅ Enhanced output formatting with emojis");
    println!("✅ Proper exit behavior matching bash version");
    println!("✅ Comprehensive edge case handling\n");

    println!("Function Signature:");
    println!("```rust");
    println!("pub fn list_clients() -> Result<(), String>");
    println!("```\n");

    println!("Usage Example:");
    println!("```rust");
    println!("use wireguard_install::client::list_clients;");
    println!("");
    println!("match list_clients() {{");
    println!("    Ok(()) => println!(\"Clients listed successfully\"),");
    println!("    Err(e) => eprintln!(\"Error: {{}}\", e),");
    println!("}}");
    println!("```\n");

    println!("Sample Output (with clients):");
    println!("📋 WireGuard Client List");
    println!("");
    println!("Found 3 clients:");
    println!("  1) laptop-client");
    println!("  2) phone-client");
    println!("  3) work-desktop");
    println!("");

    println!("Sample Output (no clients):");
    println!("");
    println!("You have no existing clients!");
    println!("(Program exits with code 1)\n");

    println!("=== Implementation Complete ===");
}
