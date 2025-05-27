# Markdown Oxide Development Guide

## Build Commands
```bash
# Build the project
cargo build

# Run the binary
cargo run

# Run tests
cargo test

# Check code style and common issues
cargo clippy

# Format code
cargo fmt
```

## Code Style Guidelines
- **Imports**: Group by standard lib, external crates, then internal modules
- **Naming**: Use snake_case for variables/functions, CamelCase for types/traits
- **Error Handling**: Use `anyhow` for general errors, custom errors for specific cases
- **Documentation**: Document public APIs with rustdoc comments
- **Types**: Prefer strong typing with custom types over primitives
- **Async**: Use `async/await` consistently, avoid mixing with direct futures

## Development Approach
- **Planning**: For tasks that involve multiple steps or components, first state your implementation plan before writing code
- **Break Down Complex Tasks**: Identify the main components, dependencies, and sequence of implementation
- **State Your Plan**: Clearly outline:
  - What files need to be created or modified
  - The order of implementation
  - Key design decisions or trade-offs
- **Then Implement**: After stating the plan, proceed with the actual code implementation

## Project Structure
- `src/vault/`: Core data management
- `src/completion/`: Editor completion providers
- `src/tokens.rs`: Markdown token parsing
- `src/main.rs`: Entry point and LSP server setup

## MCP Integration
MCP (Model Context Protocol) server implementation is in `src/mcp.rs`. Use this to access AI service integrations with Claude and other MCP-compatible clients.

For more information on MCP, see: https://modelcontextprotocol.io/llms-full.txt