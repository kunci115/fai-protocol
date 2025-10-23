# FAI Protocol

A decentralized version control system for AI models, built on Rust.

## 🎉 MAJOR MILESTONE: Full Distributed Version Control Over P2P

**Status**: ✅ **COMPLETE** - All core functionality implemented and working smoothly

### ✅ All Commands Working:
**Repository Management:**
- `init` - Initialize new FAI repositories
- `add` - Add model files to staging
- `commit` - Create commits with messages
- `log` - View commit history
- `status` - Check repository status

**Network Operations:**
- `serve` - Start P2P server for file sharing
- `fetch` - Direct file transfer between peers
- `push` - Share commits with peers
- `pull` - Receive commits from peers
- `clone` - Clone entire repositories
- `diff` - Compare commits/versions
- `peers` - Discover network peers
- `chunks` - Manage multi-chunk files

### ✅ Features Implemented:
- **P2P Commit Sharing** - Decentralized version control
- **Distributed Version History** - Complete git-like functionality
- **Repository Cloning** - Full repository replication
- **Commit Comparison** - Support for short hashes
- **Reliable Peer Discovery** - mDNS with fallback options
- **Request-Response Protocol** - Efficient P2P communication
- **Multi-chunk File Support** - Handle large AI models
- **Content Deduplication** - Storage efficiency
- **Thread-Safe Storage** - Concurrent access support

### 🔧 Issues Resolved:
- Database integration and schema management
- Network timeout and reliability issues
- Peer discovery consistency
- Protocol request/response handling
- Type safety and Rust compilation issues

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

# View commit history
fai log

# Start serving your models to the network
fai serve

# Discover peers on the network
fai peers

# Clone a repository from a peer
fai clone <peer-id>

# Pull commits from a peer
fai pull <peer-id>

# Push commits to a peer
fai push <peer-id>
```

## Architecture

- **Storage Layer**: Content-addressed storage using BLAKE3 for integrity
- **Metadata Database**: SQLite for tracking model versions and relationships  
- **CLI Interface**: Intuitive command-line interface built with Clap
- **Async Runtime**: Built on Tokio for efficient concurrent operations
- **Network Layer**: libp2p-based peer-to-peer networking with mDNS discovery
- **Chunking System**: Automatic file chunking for large model distribution

## Available Commands

### Repository Management
- `fai init` - Initialize a new FAI repository
- `fai add <path>` - Add a model file to the repository
- `fai commit -m <message>` - Commit changes with a message
- `fai status` - Show repository status
- `fai log` - Show commit history
- `fai diff <hash1> <hash2>` - Compare two commits or versions

### Network Operations
- `fai serve` - Start server to serve chunks to other peers
- `fai peers` - Discover and list network peers
- `fai clone <peer-id>` - Clone an entire repository from a peer
- `fai pull <peer-id> [commit-hash]` - Pull commits and files from a peer
- `fai push <peer-id>` - Push commits to a peer
- `fai fetch <peer-id> <hash>` - Fetch a chunk of data from a peer
- `fai chunks <hash>` - List chunks for a multi-chunk file

## Development

FAI Protocol is written in Rust for performance and safety. The project is structured as:

- `src/main.rs` - CLI entry point and command handling
- `src/lib.rs` - Core library interface
- `src/storage/` - Content-addressed storage and chunking system
- `src/database/` - SQLite metadata management
- `src/network/` - libp2p peer-to-peer networking
- `tests/` - Integration and unit tests

## Testing

```bash
# Run all tests
cargo test

# Run specific test modules
cargo test storage::tests
cargo test database::tests

# Build and check for issues
cargo build
cargo clippy
cargo fmt
```

## License

MIT License - see LICENSE file for details.
