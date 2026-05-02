# AGENTS.md

## Project Overview
**ern** (EncryptedData's ReNamer) is a Rust TUI application for renaming files in a specified directory. Built with `crossterm` and `ratatui`.

## Tech Stack
- Rust (edition 2021)
- `ratatui` 0.29 for TUI rendering
- `crossterm` 0.28 for terminal handling
- `color-eyre` for error handling
- `regex` for pattern matching

## Commands
```bash
cargo build          # Build the project
cargo run            # Run the application
cargo test           # Run tests
cargo clippy         # Lint
cargo fmt            # Format code
```

## Code Style
- Follow standard Rust conventions (rustfmt, clippy)
- No unnecessary comments unless explicitly requested
- Use idiomatic Rust patterns
- Prefer `Result` over `panic` where appropriate

## Architecture
- TUI-based interface using ratatui
- File renaming logic with regex pattern support
- Single binary crate
