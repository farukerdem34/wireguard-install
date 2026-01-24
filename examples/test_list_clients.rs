use wireguard_install::client::list_clients;

fn main() {
    println!("Testing list_clients function...");

    match list_clients() {
        Ok(()) => println!("✓ Function completed successfully"),
        Err(e) => println!("✗ Error: {}", e),
    }
}
