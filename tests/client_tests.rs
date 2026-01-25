#[cfg(test)]
mod tests {
    use crate::client::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_remove_client_from_config_single_client() {
        let config_content = r#"[Interface]
Address = 10.8.0.1/24
PrivateKey = server_private_key_here

### Client target-client
[Peer]
PublicKey = target_client_public_key
PresharedKey = target_client_preshared_key
AllowedIPs = 10.8.0.2/32
"#;

        let expected_content = r#"[Interface]
Address = 10.8.0.1/24
PrivateKey = server_private_key_here"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), config_content).unwrap();
        remove_client_from_config(temp_file.path().to_str().unwrap(), "target-client").unwrap();

        let result_content = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(result_content.trim(), expected_content.trim());
    }

    #[test]
    fn test_remove_client_from_config_multiple_clients_with_empty_lines() {
        let config_content = r#"[Interface]
Address = 10.8.0.1/24
PrivateKey = server_private_key_here

### Client client1
[Peer]
PublicKey = client1_public_key
PresharedKey = client1_preshared_key
AllowedIPs = 10.8.0.2/32

### Client target-client
[Peer]
PublicKey = target_client_public_key
PresharedKey = target_client_preshared_key
AllowedIPs = 10.8.0.3/32

### Client client3
[Peer]
PublicKey = client3_public_key
PresharedKey = client3_preshared_key
AllowedIPs = 10.8.0.4/32
"#;

        let expected_content = r#"[Interface]
Address = 10.8.0.1/24
PrivateKey = server_private_key_here

### Client client1
[Peer]
PublicKey = client1_public_key
PresharedKey = client1_preshared_key
AllowedIPs = 10.8.0.2/32


### Client client3
[Peer]
PublicKey = client3_public_key
PresharedKey = client3_preshared_key
AllowedIPs = 10.8.0.4/32"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), config_content).unwrap();
        remove_client_from_config(temp_file.path().to_str().unwrap(), "target-client").unwrap();

        let result_content = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(result_content.trim(), expected_content.trim());
    }

    #[test]
    fn test_remove_client_from_config_multiple_clients_no_empty_lines() {
        let config_content = r#"[Interface]
Address = 10.8.0.1/24
PrivateKey = server_private_key_here
### Client client1
[Peer]
PublicKey = client1_public_key
PresharedKey = client1_preshared_key
AllowedIPs = 10.8.0.2/32
### Client target-client
[Peer]
PublicKey = target_client_public_key
PresharedKey = target_client_preshared_key
AllowedIPs = 10.8.0.3/32
### Client client3
[Peer]
PublicKey = client3_public_key
PresharedKey = client3_preshared_key
AllowedIPs = 10.8.0.4/32
"#;

        let expected_content = r#"[Interface]
Address = 10.8.0.1/24
PrivateKey = server_private_key_here
### Client client1
[Peer]
PublicKey = client1_public_key
PresharedKey = client1_preshared_key
AllowedIPs = 10.8.0.2/32
### Client client3
[Peer]
PublicKey = client3_public_key
PresharedKey = client3_preshared_key
AllowedIPs = 10.8.0.4/32"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), config_content).unwrap();
        remove_client_from_config(temp_file.path().to_str().unwrap(), "target-client").unwrap();

        let result_content = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(result_content.trim(), expected_content.trim());
    }

    #[test]
    fn test_remove_client_from_config_client_not_found() {
        let config_content = r#"[Interface]
Address = 10.8.0.1/24
PrivateKey = server_private_key_here

### Client client1
[Peer]
PublicKey = client1_public_key
PresharedKey = client1_preshared_key
AllowedIPs = 10.8.0.2/32
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), config_content).unwrap();
        remove_client_from_config(temp_file.path().to_str().unwrap(), "nonexistent-client")
            .unwrap();

        let result_content = fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(result_content.trim(), config_content.trim());
    }
}
