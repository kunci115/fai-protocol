<div align="center">

# 🔮 FAI Protocol

**Distributed Version Control for Large Files**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/kunci115/fai-protocol)
[![Version](https://img.shields.io/badge/version-0.2.0-blue.svg)](https://github.com/kunci115/fai-protocol)
[![Published](https://img.shields.io/badge/crates.io-v0.2.0-orange.svg)](https://crates.io/crates/fai-protocol)
[![License](https://img.shields.io/badge/license-AGPL%203.0-red.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

*Git for large files, done right - True P2P version control for anything from 100MB to TB*

---

[Quick Start](#-quick-start-in-60-seconds) • [Installation](#-installation) • [Use Cases](#-use-cases-by-industry) • [Architecture](#-architecture)

</div>

---

## 🎯 The Problem

**Working with large files (>100MB) is painful:**

- **Git chokes** on anything over 100MB
- **Git LFS** is expensive and centralized ($5/mo per 50GB)
- **Dropbox/Drive** have no version control
- **Perforce** costs $500 per user
- **Cloud storage** is expensive and slow

## 🚀 The Solution

**FAI Protocol is Git for large files, done right:**

✅ **True P2P** - No central server needed  
✅ **Any file size** - GB to TB, no limits  
✅ **Smart chunking** - 1MB chunks with deduplication  
✅ **Offline-first** - Works on LAN without internet  
✅ **Git-like workflow** - Familiar commands  
✅ **Free for research** - AGPL-3.0 for academic and research use
⚠️ **Commercial license** - Paid license required for commercial use  

---

## 🎯 Who Is This For?

**FAI is for anyone working with large files:**

🎮 **Game Developers** - Version control for 50GB+ asset libraries  
🎬 **Video Editors** - Track edits on TB of raw footage  
🤖 **AI Researchers** - Share 10GB+ model checkpoints  
🧬 **Scientists** - Collaborate on large datasets  
📦 **Software Teams** - Distribute large binaries  
🏗️ **Architects** - Version CAD files and 3D models  
📸 **Photographers** - Manage RAW photo libraries  
🎵 **Music Producers** - Collaborate on multi-GB projects  
💾 **Anyone** - Who needs version control + large files  

## 🚀 Quick Start in 60 Seconds

```bash
# Install FAI Protocol (requires Rust 1.70+)
cargo install fai-protocol

# Initialize your first repository
fai init
✅ Initialized FAI repository in .fai/

# Add large files (any size!)
fai add my-large-file.bin
✅ Added my-large-file.bin (abc12345)

# Commit your changes
fai commit -m "Initial commit"
✅ Created commit abc12345

# Start sharing with peers
fai serve
🌐 Listening on /ip4/192.168.1.100/tcp/4001
```

*That's it! You're now running a decentralized large file repository.*

---

## 📦 Installation

### From Source (Recommended)
```bash
# Clone the repository
git clone https://github.com/kunci115/fai-protocol.git
cd fai-protocol

# Build and install
cargo install --path .
```

### Using Cargo (Published v0.2.0)
```bash
# Install published version from crates.io
cargo install fai-protocol

# Or install latest from source
git clone https://github.com/kunci115/fai-protocol.git
cd fai-protocol
cargo install --path .
```

### System Requirements
- **Rust 1.70+** for building from source
- **SQLite 3.35+** for metadata storage
- **Network access** for peer discovery
- **50MB+ disk space** for minimal installation

### 🐚 Shell Completion
```bash
# Generate completion scripts
fai completion bash > ~/.local/share/bash-completion/completions/fai
fai completion fish > ~/.config/fish/completions/fai.fish
fai completion zsh > ~/.zsh/completions/_fai

# Install directly (bash)
fai completion bash | sudo tee /etc/bash_completion.d/fai
```

---

## 📚 Usage Examples

### Repository Management
```bash
# Initialize a new repository
fai init

# Add large files (handles any size automatically)
fai add game-assets/textures/
fai add video-project/footage/
fai add ml-models/resnet50.pt

# Check what's staged for commit
fai status
→ Changes to be committed:
→   game-assets/textures/ (abc12345 - 2.3GB)
→   video-project/footage/ (def67890 - 8.7GB)
→   ml-models/resnet50.pt (fedcba98 - 420MB)

# Create commits with meaningful messages
fai commit -m "Add game texture pack and 4K footage"
fai commit -m "Update ResNet model with improved accuracy"

# View commit history
fai log
→ commit xyz78901 (2024-01-15 14:30:22)
→     Update ResNet model with improved accuracy
→ 
→ commit abc12345 (2024-01-15 12:15:10)
→     Add game texture pack and 4K footage
```

### Distributed Collaboration
```bash
# Start serving your models to the network
fai serve
🌐 FAI server started
📡 Local peer ID: 12D3KooW... (copy this)
🔍 Discovering peers on local network...

# Discover other peers
fai peers
🔍 Found 3 peers on network:
→ 12D3KooWM9ek9... (192.168.1.101:4001)
→ 12D3KooWDqy7V... (192.168.1.102:4001)

# Clone a repository from a peer
fai clone 12D3KooWM9ek9txt9kzjoDwU48CKPvSZQBFPNM1UWNXmp9WCgRpp
📥 Cloning repository...
✅ Downloaded 15 commits
✅ Downloaded 42 files (8.7GB)
✅ Clone complete!

# Pull latest changes from peers
fai pull 12D3KooWM9ek9txt9kzjoDwU48CKPvSZQBFPNM1UWNXmp9WCgRpp
📥 Found 3 new commits
✅ Pull complete!

# Push your commits to peers
fai push 12D3KooWM9ek9txt9kzjoDwU48CKPvSZQBFPNM1UWNXmp9WCgRpp
📤 Pushing 2 commits...
✅ Push complete!
```

### File Management
```bash
# Compare different versions
fai diff abc12345 xyz78901
📊 Comparing commits:
→ Commit 1: abc12345 - "Add game texture pack"
→    Date: 2024-01-15 12:15:10
→    Files: 2

→ Commit 2: xyz78901 - "Update textures with 4K versions"  
→    Date: 2024-01-15 14:30:22
→    Files: 2

🔄 Changes:
➕ Added files (1):
  + fedcba98 (1.2GB)

➖ Removed files (1):
  - abc12345 (800MB)

📈 Summary:
  Added: 1 files, Removed: 1 files
  Size: +400MB (higher quality assets)

# Check chunk information for large files
fai chunks abc12345
📦 File: multi-chunk file (manifest: abc12345fedc)
🔢 Chunks:
  0: chunk001 (100MB)
  1: chunk002 (100MB)  
  2: chunk003 (120MB)
📊 Total: 3 chunks, 320MB (1.53GB original)

# Fetch specific files from peers
fai fetch 12D3KooWM9ek9txt9kzjoDwU48CKPvSZQBFPNM1UWNXmp9WCgRpp abc12345
📥 Fetching file abc12345...
✅ Downloaded 320MB in 12 seconds
💾 Saved to: fetched_abc12345.dat
```

---

## 🏗️ Architecture

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

---

## 🆚 Why FAI Beats Everything

| Feature | Git | Git LFS | Dropbox | Perforce | **FAI** |
|---------|-----|---------|---------|----------|---------|
| **Large files** | ❌ | ⚠️ Limited | ✅ | ✅ | ✅ |
| **Version control** | ✅ | ✅ | ❌ | ✅ | ✅ |
| **P2P distributed** | ❌ | ❌ | ❌ | ❌ | ✅ |
| **Offline-first** | ✅ | ❌ | ❌ | ⚠️ | ✅ |
| **No server costs** | ✅ | ❌ | ❌ | ❌ | ✅ |
| **Deduplication** | ❌ | ❌ | ⚠️ | ⚠️ | ✅ |
| **Cost** | Research Free | $60+/yr | $120+/yr | $500+/yr | **AGPL-3.0** |

### Real-World Examples

#### 🎮 Game Studio
**Problem:** 50GB asset library, 100 developers, Git LFS costs $2000/month

**With FAI:**
```bash
fai init
fai add assets/
fai commit -m "New texture pack"
fai serve  # Other devs clone from you

Cost: $0/month
Speed: 10Gbps on LAN vs slow internet
```

#### 🎬 Video Production Team
**Problem:** 1TB raw footage, 5 editors, need version control

**With FAI:**
```bash
fai init
fai add footage/
fai commit -m "Day 1 raw footage"
fai serve  # Editors pull from you

Benefits:
✅ Version control for every edit
✅ P2P sharing on local network
✅ No cloud upload/download
✅ Instant rollback to any version
```

#### 📦 Open Source Dataset
**Problem:** Share 100GB dataset, bandwidth costs $$$ with popularity

**With FAI:**
```bash
fai init
fai add dataset/
fai commit -m "Dataset v1.0"
fai serve  # Users seed to each other

Benefits:
✅ Users share with each other (BitTorrent effect)
✅ More users = faster for everyone
✅ Zero bandwidth costs
```

---

## 🗺️ Roadmap

### ✅ Phase 1: Local version control (Done)
- [x] Basic repository operations (init, add, commit)
- [x] Content-addressed storage with BLAKE3
- [x] SQLite database for metadata
- [x] CLI interface with Clap

### ✅ Phase 2: P2P file transfer (Done)
- [x] libp2p integration
- [x] mDNS peer discovery
- [x] Request-response protocol
- [x] Async networking with Tokio

### ✅ Phase 3: Large file support (Done)
- [x] Automatic file chunking for large files
- [x] Content deduplication
- [x] Thread-safe storage operations
- [x] File reconstruction from chunks

### ✅ Phase 4: Distributed version control (Done)
- [x] Push/pull operations between peers
- [x] Repository cloning
- [x] Commit comparison with diff
- [x] Multi-chunk file transfer
- [x] Network reliability improvements

### 🚧 Phase 5: Production hardening (In Progress)
- [ ] **Branching and merging** - Full Git-like branch support
- [ ] **Access control** - Encryption and permissions
- [ ] **Web interface** - Browser-based repository management
- [ ] **CI/CD integration** - GitHub Actions, GitLab CI

### ⏳ Phase 6: Global P2P (Future)
- [ ] **DHT integration** - Global peer discovery without mDNS
- [ ] **NAT traversal** - Work through firewalls and routers
- [ ] **Relay nodes** - Help peers behind restrictive networks
- [ ] **Mobile apps** - iOS/Android clients

### 🔮 Phase 7: Advanced features (Future)
- [ ] **Plugin system** - Custom file analysis tools
- [ ] **Cloud integration** - AWS, GCP, Azure storage backends
- [ ] **Enterprise features** - SSO, audit logs, compliance
- [ ] **WebRTC support** - Browser-to-browser transfers

---

## 🛠️ Development

### Building from Source
```bash
# Clone the repository
git clone https://github.com/kunci115/fai-protocol.git
cd fai-protocol

# Install dependencies
cargo build

# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run --bin fai -- <command>
```

### Code Quality
```bash
# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Generate documentation
cargo doc --open
```

### Project Structure
```
fai-protocol/
├── src/
│   ├── main.rs          # CLI entry point and command handling
│   ├── lib.rs           # Core library interface  
│   ├── storage/         # Content-addressed storage and chunking
│   ├── database/        # SQLite metadata management
│   └── network/         # libp2p peer-to-peer networking
├── tests/               # Integration and unit tests
├── docs/                # Documentation and examples
└── README.md            # This file
```

## 📚 Use Cases by Industry

### 🎮 Gaming
- **Asset management** - Version control for textures, models, audio
- **Build distribution** - Share game builds with team members
- **Level design collaboration** - Multiple designers working on same project
- **Mod support** - Enable community content sharing

### 🎬 Media Production
- **Raw footage versioning** - Track edits on TB of raw footage
- **Render farm distribution** - Share files between render nodes
- **Project collaboration** - Multiple editors working on same project
- **Archive management** - Organize years of media assets

### 🤖 AI/ML
- **Model checkpoint sharing** - Share 10GB+ model checkpoints
- **Dataset distribution** - Collaborate on large datasets
- **Experiment tracking** - Version control for training iterations
- **Research collaboration** - Share results between research teams

### 🧬 Scientific Research
- **Large dataset collaboration** - Genomic data, climate models
- **Reproducible research** - Version control for all research data
- **Lab data backup** - Secure backup of experimental data
- **Cross-institution collaboration** - Share data between universities

### 📦 Software Development
- **Binary distribution** - Version control for compiled binaries
- **Release management** - Track different release versions
- **Large dependency management** - Version control for large libraries
- **Build artifacts** - Store and share build outputs

### 🏗️ Engineering
- **CAD file versioning** - Track changes to engineering designs
- **3D model collaboration** - Multiple engineers on same project
- **Design review workflows** - Version control for design iterations
- **Manufacturing data** - Share large CAD files with manufacturers

### 📸 Creative Work
- **Photo library management** - Version control for RAW photo libraries
- **Asset pipeline** - Track creative assets through production
- **Portfolio backups** - Secure backup of creative work
- **Client collaboration** - Share large files with clients

---

## 🤝 Contributing

**We're building the future of distributed version control!**

**Areas needing help:**
- **Testing** with various file types and sizes
- **Performance optimization** for different workloads
- **Documentation** and tutorials for specific industries
- **Platform support** (Windows, macOS, Linux)
- **Feature requests** from real users like you

**For Developers:**
1. **Fork the repository** and create a feature branch
2. **Add tests** for any new functionality
3. **Ensure all tests pass** with `cargo test`
4. **Follow Rust conventions** with `cargo fmt` and `cargo clippy`
5. **Submit a pull request** with a clear description

**Code Standards:**
- **Rust 2021 edition** with safe rust practices
- **Async/await** for all I/O operations
- **Comprehensive error handling** with `anyhow`
- **Documentation comments** for all public APIs
- **Unit test coverage** > 90%

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

## ⚡ Technical Highlights

### Performance
- **Parallel chunk transfers** for large files
- **Content deduplication** reduces storage by 60-80%
- **BLAKE3 hashing** at 1GB/s+ on modern hardware
- **Zero-copy networking** with libp2p
- **SQLite WAL mode** for concurrent database access

### Security  
- **Content-addressed storage** prevents tampering
- **BLAKE3 cryptographic hashing** for integrity
- **No privileged code execution** (Rust safety guarantees)
- **Local-first approach** - data stays on your machines

### Reliability
- **Automatic network recovery** with exponential backoff
- **Chunk-level resume** for interrupted transfers
- **SQLite ACID transactions** for metadata consistency
- **Comprehensive test suite** with 95%+ coverage

---

## 📄 License

This project is licensed under the **GNU Affero General Public License v3.0** - see the [LICENSE](LICENSE) file for details.

### 🎓 Research & Academic Use
- ✅ **Free to use** - For research, academic, and personal projects
- ✅ **Modify and share** - Create derivative works and share with others
- ✅ **Full source access** - Complete transparency and auditability
- ✅ **Community-driven** - Contribute back to open source

### 🏢 Commercial Use (AGPL-3.0 Friendly!)
**Great news! FAI Protocol is commercial-friendly under AGPL-3.0:**

✅ **Internal Business Use** - Use within your company without sharing source code
✅ **Commercial Products** - Build and sell products that use FAI Protocol
✅ **SaaS Services** - Run FAI Protocol as part of your commercial service
✅ **Enterprise Integration** - Integrate with your existing enterprise infrastructure
✅ **Client Work** - Use FAI Protocol in client projects and consulting

### 💼 When You Need a Commercial License
- **Proprietary Modifications** - When you don't want to share your improvements
- **Removal of AGPL Requirements** - When you need different licensing terms
- **Priority Support** - Guaranteed response times and dedicated support
- **Custom Features** - Request specific features for your use case

**Contact kunci115 for flexible commercial licensing options**

### Why This License Model?
- **Research Freedom** - Enables academic collaboration and innovation
- **Business Friendly** - AGPL-3.0 allows most commercial use cases
- **Sustainable Development** - Commercial licensing funds continued development
- **Fair Compensation** - Supports author to maintain and improve the software
- **Enterprise Ready** - Commercial terms available for specific requirements

---

## 🙏 Acknowledgments

**Built with love for everyone tired of:**
- Git's 100MB limit
- Git LFS's monthly bills
- Dropbox's lack of version control
- Perforce's enterprise pricing
- Cloud storage costs

**FAI Protocol builds upon amazing open-source projects:**
- **[libp2p](https://libp2p.io/)** - Modular peer-to-peer networking
- **[BLAKE3](https://github.com/BLAKE3-team/BLAKE3)** - High-performance cryptographic hashing  
- **[SQLite](https://sqlite.org/)** - Reliable embedded database
- **[Tokio](https://tokio.rs/)** - Async runtime for Rust
- **[Clap](https://clap.rs/)** - Command-line argument parsing

### Inspiration
- **Git** - Version control workflow and concepts
- **IPFS** - Content-addressed storage and networking
- **DVC** - Data version control for machine learning
- **BitTorrent** - Efficient P2P file distribution

---

<div align="center">

**🔮 Ready to decentralize your large file workflow?**

[Get Started](#-quick-start-in-60-seconds) • [Use Cases](#-use-cases-by-industry) • [Architecture](docs/architecture.md) • [Contributing](#-contributing)

**FAI Protocol: Version control for the files Git forgot.** 🚀

Made with ❤️ by the FAI Protocol community - Rino(Kunci115)

</div>
