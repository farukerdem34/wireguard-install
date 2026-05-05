## Build, Lint, and Test
- Build: `cargo build` (debug), `cargo build --release`
- Test: `cargo test` or `cargo test <test_name>`
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Entrypoints and Flow
- App entrypoint is `src/main.rs` -> `initial_check()` in `src/initialization.rs`.
- If `/etc/wireguard/params` exists, it shows the management menu; otherwise it installs WireGuard.

## Runtime Requirements (from code)
- The app reads/writes `/etc/wireguard/*`, runs system commands (`wg`, package managers, `systemctl`/`rc-service`, `iptables`/`firewalld`), and assumes Linux with root privileges.
