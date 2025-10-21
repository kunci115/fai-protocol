# FAI Protocol

A decentralized version control system for AI models, built on Rust.

## Vision

FAI Protocol enables AI developers to version, share, and track machine learning models in a decentralized manner. By using content-addressed storage with BLAKE3 hashing and distributed metadata management, FAI provides:

- **Immutable model storage** - Models are stored by content hash, ensuring integrity
- **Decentralized sharing** - No central authority required for model distribution
- **Version tracking** - Complete history of model changes and training iterations
- **Collaboration** - Multiple contributors can work on model development together

## Quick Start

```bash
# Initialize a new FAI repository
fai init

# Add a trained model
fai add models/my_model.onnx

# Commit with a descriptive message
fai commit "Added initial ResNet-50 model"

# Check repository status
fai status
```

## Architecture

- **Storage Layer**: Content-addressed storage using BLAKE3 for integrity
- **Metadata Database**: SQLite for tracking model versions and relationships  
- **CLI Interface**: Intuitive command-line interface built with Clap
- **Async Runtime**: Built on Tokio for efficient concurrent operations

## Development

FAI Protocol is written in Rust for performance and safety. The project is structured as:

- `src/main.rs` - CLI entry point and command handling
- `src/lib.rs` - Core library interface
- `src/storage/` - Storage and metadata management
- `Cargo.toml` - Project configuration and dependencies

## License

MIT License - see LICENSE file for details.
