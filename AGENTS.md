# AGENTS.md

## Build, Lint, and Test Commands
This repository is a Rust project. Below are some useful commands for development and testing:

### Build Project
```bash
cargo build    # Build the project in debug mode
cargo build --release   # Build the project in release mode
```

### Test Commands
#### Run All Tests
```bash
cargo test     # Runs all the tests
```

#### Run a Specific Test
```bash
cargo test <test_name>  # Replace <test_name> with the test's identifier
```

### Linting
This project adheres to the `clippy` linting rules.
```bash
cargo clippy  # Run clippy linting
```

## Code Style Guidelines
To ensure consistency and maintainability, follow these guidelines while contributing to this repository.

### Formatting
Use `rustfmt` for code formatting. This ensures that your code adheres to the Rust community style guide.
```bash
cargo fmt  # Automatically format code using rustfmt
```

### Import Rules
- Group external crate imports at the top, followed by internal modules.
- Use alphabetized imports for clarity.
- Prefer relative paths while importing modules within the crate.
- Example:
```rust
use crate::utils::set_permissions_recursive;
use dialoguer::{Confirm, Input};  // External imports
use std::fs;                     // Standard library imports
```

### Naming Conventions
- Use snake_case for function names, variables, and file/module names.
- Use CamelCase for structs, enums, and traits.
- Constants should be in SCREAMING_SNAKE_CASE.
- Keep names descriptive yet concise.

### Types
- Prefer explicit size types (e.g., `u8`, `i32`) over generic `int`/`uint`.
- Avoid unnecessary type annotations unless required for clarity.
- Leverage Rust's type inference as appropriate.

### Error Handling
- Use the `Result` type for recoverable errors.
- For common operations, return `Result<T, String>` as consistent with the codebase.
- An Example:
```rust
fn generate_key() -> Result<String, String> {
    let key_output = Command::new("generate-key").output().map_err(|e| format!("Failed: {}", e))?;
    String::from_utf8(key_output.stdout).map_err(|e| format!("Invalid UTF8: {}", e))
}
```
### File Permissions
- Utility functions like modifying permissions using `set_permissions_recursive` have already been written.

### Error Logging
- Print user-friendly error messages on failure and log significant steps e.g.: Running or Install pending loops etc
.

### Comments and Documentation
- Include module-level and function-level documentation (`///`).
- For multiline explanations, inline comments (`/** */` or //!).

--- End