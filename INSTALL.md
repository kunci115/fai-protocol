# FAI Protocol Installation Guide

## 🚀 Quick Installation

### Option 1: Install from Crates.io (Recommended)

```bash
cargo install fai-protocol
```

### Option 2: Install from Source

```bash
git clone https://github.com/kunci115/fai-protocol.git
cd fai-protocol
cargo install --path .
```

### Option 3: Development Installation

```bash
git clone https://github.com/kunci115/fai-protocol.git
cd fai-protocol
cargo build --release
```

The binary will be available at `./target/release/fai`.

## 📋 Requirements

- **Rust**: 1.70.0 or newer
- **Operating System**: Linux, macOS, or Windows
- **Network**: For peer discovery and distributed operations

## ✅ Verify Installation

After installation, verify that FAI Protocol is working:

```bash
fai --version
fai --help
```

## 🧪 Test Your Installation

Run the comprehensive test suite to ensure everything works correctly:

```bash
# Test from source (if cloned from git)
cd fai-protocol
cargo test --test integration_tests

# Should show all tests passing:
# running 5 tests
# test test_basic_repository_workflow ... ok
# test test_data_integrity ... ok
# test test_multiple_file_operations ... ok
# test test_error_handling ... ok
# test test_branch_operations ... ok
#
# test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 🎯 Quick Start

1. **Initialize a repository:**
   ```bash
   mkdir my-project && cd my-project
   fai init
   ```

2. **Add files:**
   ```bash
   echo "Hello FAI!" > README.md
   fai add README.md
   ```

3. **Make a commit:**
   ```bash
   fai commit -m "Initial commit"
   ```

4. **Start serving:**
   ```bash
   fai serve
   ```

5. **Clone from another machine:**
   ```bash
   fai clone <peer-id> my-project-clone
   ```

## 🔧 Advanced Installation

### Custom Installation Directory

```bash
cargo install fai-protocol --root ~/.local
export PATH="$HOME/.local/bin:$PATH"
```

### Enable Shell Completion

#### Bash
```bash
fai completion bash > ~/.local/share/bash-completion/completions/fai
# Or install directly
fai completion bash | sudo tee /etc/bash_completion.d/fai
```

#### Fish
```bash
fai completion fish > ~/.config/fish/completions/fai.fish
```

#### Zsh
```bash
fai completion zsh > ~/.zsh/completions/_fai
```

## 🐳 Docker Installation

Create a `Dockerfile`:

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/fai /usr/local/bin/
ENTRYPOINT ["fai"]
```

Build and run:

```bash
docker build -t fai-protocol .
docker run -it -v $(pwd):/workspace fai-protocol init
```

## 🔍 Troubleshooting

### Permission Denied

```bash
chmod +x ~/.cargo/bin/fai
```

### Binary Not Found

Add cargo bin directory to your PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
```

### Compilation Issues

Ensure you have the required system dependencies:

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install build-essential pkg-config libssl-dev
```

**macOS:**
```bash
xcode-select --install
brew install openssl
```

**Windows:**
Install [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

### Network Issues

If you encounter network-related issues, ensure your firewall allows:
- Outbound TCP connections
- mDNS (Multicast DNS) for local peer discovery
- Ports in the range 40000-65535 for P2P communication

## 📚 Next Steps

- Check the [API Documentation](https://docs.rs/fai-protocol) for integration
- Read the [Architecture Guide](docs/architecture.md) for technical details
- Visit the [GitHub Repository](https://github.com/kunci115/fai-protocol) for issues and contributions

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

FAI Protocol is licensed under the **GNU Affero General Public License v3.0**. See [LICENSE](LICENSE) for details.

### 🎓 Research & Academic Use
- ✅ **Free to use** for research, academic, and personal projects
- ✅ **Modify and share** with the open source community

### 🏢 Commercial Use
- ⚠️ **Commercial license required** for revenue-generating applications
- 💰 **Contact kunci115** to purchase a commercial license

This dual-licensing model enables free academic use while supporting sustainable development through commercial licensing.