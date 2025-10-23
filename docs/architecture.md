# 🏗️ FAI Protocol Architecture

This document provides a detailed technical overview of the FAI Protocol architecture, design decisions, and implementation details.

## Table of Contents

- [High-Level Architecture](#high-level-architecture)
- [Component Overview](#component-overview)
- [Storage Layer](#storage-layer)
- [Network Layer](#network-layer)
- [Database Layer](#database-layer)
- [CLI Interface](#cli-interface)
- [Data Flow](#data-flow)
- [Security Model](#security-model)
- [Performance Considerations](#performance-considerations)

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    FAI Protocol Architecture                │
├─────────────────────────────────────────────────────────────┤
│  CLI Interface                                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│  │    Init     │  │    Add      │  │   Commit    │       │
│  │   Status    │  │   Clone     │  │    Push     │       │
│  │     Log     │  │   Pull      │  │   Fetch     │       │
│  └─────────────┘  └─────────────┘  └─────────────┘       │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                Core Library Layer                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              FaiProtocol                           │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐│   │
│  │  │   Storage   │  │  Database   │  │   Network   ││   │
│  │  │  Manager    │  │  Manager    │  │  Manager    ││   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘│   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                  Infrastructure Layer                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│  │  libp2p P2P │  │    SQLite   │  │   BLAKE3    │       │
│  │  Networking │  │   Database  │  │   Hashing   │       │
│  │             │  │             │  │             │       │
│  │ • mDNS      │  │ • Commits   │  │ • Integrity  │       │
│  │ • TCP       │  │ • Metadata  │  │ • Dedup     │       │
│  │ • Noise     │  │ • Staging   │  │ • Fast      │       │
│  └─────────────┘  └─────────────┘  └─────────────┘       │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                Storage & Networking                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│  │  .fai/      │  │  P2P Network│  │  Chunks     │       │
│  │  objects/   │  │             │  │             │       │
│  │  db.sqlite  │  │ • Auto      │  │ • 1MB chunks│       │
│  │  HEAD       │  │   discovery │  │ • Parallel  │       │
│  │             │  │ • Direct    │  │   transfer  │       │
│  │             │  │   connect   │  │             │       │
│  └─────────────┘  └─────────────┘  └─────────────┘       │
└─────────────────────────────────────────────────────────────┘
```

## Component Overview

### Core Library (`src/lib.rs`)

The `FaiProtocol` struct is the main entry point that orchestrates all subsystems:

```rust
pub struct FaiProtocol {
    storage: StorageManager,
    database: DatabaseManager,
    network: NetworkManager,
}
```

**Key Responsibilities:**
- Coordinate between storage, database, and network components
- Provide high-level API for repository operations
- Manage repository lifecycle and state
- Handle error propagation and recovery

### CLI Interface (`src/main.rs`)

Command-line interface built with Clap that provides Git-like commands:

**Repository Management:**
- `init` - Initialize new repository
- `add` - Stage files for commit
- `status` - Show repository status
- `commit` - Create commits
- `log` - Show commit history

**Distributed Operations:**
- `clone` - Clone repository from peer
- `push` - Push commits to peers
- `pull` - Pull commits from peers
- `serve` - Start P2P server
- `peers` - List discovered peers

**File Operations:**
- `fetch` - Download specific files
- `diff` - Compare commits
- `chunks` - Show file chunk information

## Storage Layer

### Content-Addressed Storage (`src/storage/`)

The storage layer implements content-addressed storage with automatic chunking:

```rust
pub struct StorageManager {
    objects_dir: PathBuf,
}
```

**Key Features:**
- **BLAKE3 Hashing**: Every chunk is identified by its BLAKE3 hash
- **Automatic Chunking**: Files larger than 1MB are automatically split into chunks
- **Deduplication**: Identical chunks are stored only once
- **Thread-Safe**: All operations are safe for concurrent access
- **Atomic Operations**: File operations are atomic to prevent corruption

**File Operations:**
- `store_file()` - Store file with automatic chunking
- `retrieve_file()` - Reconstruct files from chunks
- `store_chunk()` - Store individual chunks
- `retrieve_chunk()` - Retrieve individual chunks
- `get_file_info()` - Get file metadata

**Storage Structure:**
```
.fai/objects/
├── 48/
│   └── 8de202f73bd976de4e7048f4e1f39a776d86d582b7348ff53bf432b987fca8
├── 9b/
│   └── c3a10f284690bdead750dda5be6bd4675f28dac928e19d446d082918718acb
└── manifests/
    └── abc123fedc456...
```

## Network Layer

### P2P Networking (`src/network/`)

The network layer implements peer-to-peer communication using libp2p:

```rust
pub struct NetworkManager {
    swarm: Swarm<Behaviour>,
    peers: Arc<Mutex<HashMap<PeerId, Vec<Multiaddr>>>>,
    response_senders: Arc<Mutex<HashMap<OutboundRequestId, ResponseSender>>>,
}
```

**libp2p Behaviors:**
- **mDNS**: Automatic peer discovery on local networks
- **TCP**: Reliable transport for peer communication
- **Noise Protocol**: Encrypted and authenticated communication
- **Yamux**: Stream multiplexing over single connection
- **Request-Response**: Simple RPC-style communication
- **CBOR**: Efficient binary serialization

**Network Operations:**
- `start_listening()` - Start listening for incoming connections
- `connect_to_peer()` - Connect to specific peer
- `discover_peers()` - Discover peers via mDNS
- `send_request()` - Send request to peer
- `poll_events()` - Process network events

**Peer Discovery:**
1. **mDNS Discovery**: Automatic discovery on local networks
2. **File-based Discovery**: Load peers from files for testing
3. **Direct Connection**: Connect to specific peer IDs
4. **Bootstrap**: Load saved peers from previous sessions

**Communication Protocol:**
- Uses CBOR for serialization
- Request-response pattern for API calls
- Timeout-based error handling
- Automatic retry on network failures

## Database Layer

### SQLite Metadata (`src/database/`)

The database layer manages metadata using SQLite:

```rust
pub struct DatabaseManager {
    connection: Arc<Mutex<Connection>>,
}
```

**Database Schema:**
```sql
-- Staged files for next commit
CREATE TABLE staged_files (
    path TEXT PRIMARY KEY,
    hash TEXT NOT NULL,
    size INTEGER NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Commit history
CREATE TABLE commits (
    hash TEXT PRIMARY KEY,
    message TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    parent_hash TEXT
);

-- Files in each commit
CREATE TABLE commit_files (
    commit_hash TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    PRIMARY KEY (commit_hash, file_hash),
    FOREIGN KEY (commit_hash) REFERENCES commits(hash)
);

-- Multi-chunk file manifests
CREATE TABLE manifests (
    hash TEXT PRIMARY KEY,
    total_size INTEGER NOT NULL,
    chunk_count INTEGER NOT NULL
);

-- Individual chunks in manifests
CREATE TABLE manifest_chunks (
    manifest_hash TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    chunk_hash TEXT NOT NULL,
    chunk_size INTEGER NOT NULL,
    PRIMARY KEY (manifest_hash, chunk_index),
    FOREIGN KEY (manifest_hash) REFERENCES manifests(hash)
);
```

**Database Operations:**
- Repository initialization and schema setup
- Staging area management for commits
- Commit history tracking
- File manifest and chunk tracking
- ACID transactions for data consistency

## CLI Interface

### Command Processing (`src/main.rs`)

The CLI interface provides Git-like commands using Clap:

```rust
#[derive(Parser)]
#[command(name = "fai")]
#[command(about = "A decentralized version control system for large files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
```

**Command Categories:**

**Local Operations:**
- Repository management (init, status, log)
- File operations (add, commit, fetch, diff, chunks)

**Distributed Operations:**
- Network management (serve, peers)
- Synchronization (clone, push, pull)

**Features:**
- Comprehensive error handling with user-friendly messages
- Progress bars for long-running operations
- Colored output for better readability
- Shell completion support (bash, fish, zsh)

## Data Flow

### File Addition Workflow

1. **File Staging** (`fai add`):
   ```
   File → BLAKE3 Hash → Check if chunk exists → Store chunk → Add to staging area
   ```

2. **File Storage**:
   - Files > 1MB are split into chunks
   - Each chunk is stored by its BLAKE3 hash
   - Manifest created to track chunk order
   - Deduplication prevents storing duplicate chunks

### Commit Creation Workflow

1. **Commit Process** (`fai commit`):
   ```
   Staged files → Create commit record → Store in commits table → Update HEAD pointer
   ```

2. **Metadata Storage**:
   - Commit hash calculated from content
   - Commit message and timestamp stored
   - File list and metadata stored in commit_files table
   - Parent-child relationships tracked

### P2P Synchronization Workflow

1. **Peer Discovery**:
   ```
   mDNS broadcast → Discover peers → Exchange peer info → Establish connections
   ```

2. **Repository Cloning** (`fai clone`):
   ```
   Peer connection → Request commit history → Download required chunks → Reconstruct files
   ```

3. **Push/Pull Operations**:
   - Compare commit histories
   - Transfer missing commits and chunks
   - Update local database and storage

## Security Model

### Cryptographic Security

**BLAKE3 Hashing:**
- Cryptographically secure hash function
- 256-bit output for collision resistance
- Fast hashing at 1GB/s+ on modern hardware
- Used for both integrity and deduplication

**Network Security:**
- **Noise Protocol** for encrypted communication
- **Peer ID** derived from cryptographic keys
- **Authenticated connections** prevent man-in-the-middle attacks
- **Private by design** - no central servers to compromise

### Content Integrity

**Content-Addressed Storage:**
- Each chunk identified by cryptographic hash
- Any corruption detected immediately on retrieval
- Automatic deduplication prevents data duplication
- Tamper-evident storage

**Commit Integrity:**
- Commits form a Merkle-like structure
- Parent-child links ensure history integrity
- Any modification breaks hash chain
- Immutable commit history

### Privacy Considerations

**Local-First Approach:**
- Data stays on local machines by default
- Only shared when explicitly published
- P2P communication can be limited to local networks
- No tracking or telemetry

**Network Privacy:**
- Peer IDs are cryptographic, not personal
- mDNS limited to local network
- Direct connections require explicit peer IDs
- No central servers to monitor usage

## Performance Considerations

### Storage Performance

**Chunking Strategy:**
- 1MB chunks optimize between deduplication and overhead
- Parallel chunk processing for large files
- Sequential storage for efficient disk access
- Compressed storage reduces disk usage

**Database Optimization:**
- SQLite WAL mode for concurrent access
- Indexes on frequently queried columns
- Prepared statements for repeated operations
- Connection pooling for multiple operations

### Network Performance

**Parallel Transfers:**
- Multiple chunks transferred concurrently
- Connection pooling to reduce overhead
- Automatic retry on network failures
- Adaptive timeout based on network conditions

**Bandwidth Efficiency:**
- CBOR serialization for compact messages
- Delta transfer for commit synchronization
- Chunk deduplication reduces transfer size
- Local network prioritization

### Memory Management

**Streaming Processing:**
- Large files processed in chunks
- Memory-mapped file operations
- Bounded memory usage regardless of file size
- Efficient async/await patterns

**Resource Cleanup:**
- Automatic cleanup of temporary files
- Connection pooling with timeout
- Proper error handling prevents resource leaks
- Memory usage scales with operation size, not repository size

## Implementation Details

### Error Handling

**Error Hierarchy:**
```rust
pub enum FaiError {
    StorageError(StorageError),
    NetworkError(NetworkError),
    DatabaseError(DatabaseError),
    IoError(std::io::Error),
}
```

**Recovery Strategies:**
- Network timeouts with exponential backoff
- Database transaction rollback on errors
- File operation atomicity
- Graceful degradation on network issues

### Concurrency Model

**Thread Safety:**
- Arc<Mutex<T>> for shared state protection
- Async/await for non-blocking I/O
- Atomic operations for counters
- Lock-free data structures where possible

**Async Runtime:**
- Tokio for async operations
- Structured concurrency with tasks
- Proper cancellation handling
- Resource cleanup on task completion

### Configuration

**Default Settings:**
- Chunk size: 1MB (optimized for most workloads)
- Network timeout: 30 seconds
- Connection pool: 10 connections
- Database connection: 1 connection with WAL mode

**Tunable Parameters:**
- Chunk size for different file types
- Network timeouts for various environments
- Peer discovery intervals
- Storage compression levels

---

This architecture document provides a comprehensive overview of FAI Protocol's design and implementation. For specific code details, refer to the source code and inline documentation.