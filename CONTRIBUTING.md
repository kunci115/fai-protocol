# 🤝 Contributing to FAI Protocol

**We're building the future of distributed version control!**
We welcome contributions from the community. FAI Protocol is a decentralized version control system designed for large files and AI models.

## 🎯 What We're Building

FAI Protocol is Git for large files, done right. We're solving the problem that traditional Git can't handle files >100MB, while existing solutions like Git LFS are expensive and centralized.

**Current Focus Areas (v0.4.1):**
- ✅ **Complete branch management** - Git-like branching with `fai branch`, `fai checkout`
- ✅ **Commit amend functionality** - Fix last commits with `fai commit-amend`
- ✅ **Web interface** - Browser-based repository management
- ✅ **Modular architecture** - Service-oriented design for maintainability

**Next Milestones:**
- 🚧 **Merge operations** - `fai merge` with conflict resolution
- 🚧 **Advanced Git features** - `fai rebase`, `fai cherry-pick`
- 🚧 **Security & authentication** - User management and access control
- 🚧 **Global P2P** - DHT integration and NAT traversal

## 🚀 Quick Start

1. **Fork the repository** on GitHub
   ```bash
   git clone https://github.com/kunci115/fai-protocol.git
   cd fai-protocol
   ```

2. **Create a feature branch**
   ```bash
   git checkout -b feature-name
   ```

3. **Make your changes**
   - Write code following Rust best practices
   - Add tests for new functionality
   - Update documentation as needed

4. **Test your changes**
   ```bash
   cargo test
   cargo test --test integration_tests
   cargo build
   ```

5. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add your feature description"
   ```

6. **Push and create a pull request**
   ```bash
   git push origin feature-name
   ```

## 🏗️ Development Areas (v0.4.1 Focus)

We welcome contributions in several key areas, with current focus on version control features:

### 🌿 Version Control Features (Priority)
- **Merge operations** - Implement `fai merge` with conflict resolution
- **Advanced Git features** - Add `fai rebase -i`, `fai cherry-pick`
- **Branch visualization** - Visual branch history and merge graphs
- **Commit editing** - Interactive commit history management

### 🌐 Web Interface Enhancement
- **Interactive UI** - Add file staging, commit, and branch management
- **Real-time updates** - WebSocket support for live status updates
- **Authentication** - User login and session management
- **Mobile responsive** - Better mobile experience

### 📦 Core Library
- **Storage optimization** - Improve chunking and deduplication algorithms
- **Network performance** - Enhance P2P transfer speeds and reliability
- **Database efficiency** - Optimize SQLite operations and queries
- **CLI usability** - Improve user experience and error handling

### 🔒 Security & Authentication
- **User management** - Implement security service features
- **Access control** - Repository permissions and team management
- **Encryption** - File encryption and secure transfers
- **Audit logging** - Track repository access and changes

### 🔧 Tooling & Ecosystem
- **IDE integrations** - VS Code, IntelliJ, Vim plugins
- **CI/CD pipelines** - GitHub Actions, testing workflows
- **Documentation** - Improve guides, API docs, and examples
- **Performance testing** - Benchmarks and profiling tools

### 🎯 Use Cases & Examples
- **Industry integrations** - Game dev, video editing, AI/ML workflows
- **Migration tools** - Import from Git, Mercurial, Perforce
- **Web interface** - Browser-based repository management
- **Mobile apps** - iOS/Android clients

## 📋 Code Standards

### 🏗️ Architecture Guidelines (v0.4.1)
FAI Protocol follows a **service-oriented modular architecture**:

```
src/
├── services/              # Service modules (v0.4.1)
│   ├── cli_service.rs     # CLI command handling and operations
│   ├── branch_service.rs  # Branch management operations
│   ├── web_service.rs     # Web interface and REST API
│   └── security_service.rs # Authentication and encryption
├── storage/               # Content-addressed storage
├── database/              # SQLite metadata management
├── network/               # libp2p P2P networking
└── main.rs               # CLI entry point
```

**Service Design Principles:**
- **Single responsibility** - Each service handles one domain
- **Clean interfaces** - Well-defined APIs between services
- **Error propagation** - Proper error handling across service boundaries
- **Testability** - Services can be tested independently

**Adding New Services:**
1. Create service module in `src/services/`
2. Implement service with clear public interface
3. Add service to `src/services/mod.rs`
4. Include comprehensive tests
5. Update documentation

### 🦀 Rust Requirements
- **Rust 2021 Edition** with modern language features
- **Safe Rust** - Avoid unsafe code where possible
- **Error handling** - Use `Result` and `anyhow` for proper error management
- **Async/await** - Use Tokio runtime for all I/O operations
- **Service-oriented design** - Follow modular architecture patterns
- **Documentation** - Include comprehensive `cargo doc` comments

### 🧪 Testing Requirements
- **Unit tests** - Test individual functions and modules
- **Integration tests** - Test component interactions with our comprehensive suite:
  - `test_basic_repository_workflow` - Core repository operations (init, add, commit, status, log)
  - `test_data_integrity` - File integrity verification and content validation
  - `test_multiple_file_operations` - Multiple file handling and batch operations
  - `test_error_handling` - Graceful error recovery and edge cases
  - `test_branch_operations` - Basic branch management functionality
- **Performance tests** - Benchmark critical operations
- **Network tests** - Test P2P functionality with mock peers

**Test Requirements:**
- All integration tests must pass before merging
- New features should include corresponding integration tests
- Tests must be isolated and not interfere with each other
- Use `cargo test --test integration_tests` to run the full suite

### 📖 Documentation Requirements
- **Code comments** - Document all public APIs and complex logic
- **Examples** - Provide working code samples for major features
- **README updates** - Update user-facing documentation
- **CHANGELOG** - Document significant changes

## 🎯 Development Workflow

### Before Starting
1. **Check existing issues** - Look for related discussions or ongoing work
2. **Create an issue** - Describe your proposed changes
3. **Wait for feedback** - Get community input before starting

### During Development
1. **Small commits** - Make focused, atomic changes
2. **Clear messages** - Use conventional commit format:
   ```
   feat: add new feature
   fix: resolve bug description
   docs: update documentation
   test: add test coverage
   refactor: improve code structure
   ```
3. **Test frequently** - Run tests after each major change

### Before Submitting
1. **Full test suite** - Ensure `cargo test` and `cargo test --test integration_tests` pass
2. **Code formatting** - Run `cargo fmt`
3. **Linting** - Run `cargo clippy -- -D warnings`
4. **Documentation** - Update relevant docs and examples
5. **CI/CD Check** - Ensure GitHub Actions workflow will pass

### 🔄 CI/CD Workflow
FAI Protocol uses GitHub Actions for automated testing:

**Workflow Steps:**
1. **Build Verification** - Compile on Ubuntu latest
2. **Test Suite Execution** - Run unit and integration tests
3. **Integration Testing** - Verify all repository operations work correctly
4. **Publish Preparation** - Build release artifacts for publishing

**Workflow Tests:**
- Basic repository workflow (init, add, commit, status, log)
- P2P networking basic functionality
- Data integrity and consistency checks
- Multiple file operations
- Error handling verification
- Branch operations

The workflow ensures every contribution maintains the high quality and reliability standards of FAI Protocol.

## 🏷️ License & Contributors

### AGPL-3.0 Dual Licensing
- **Research/Academic** - Free use under AGPL-3.0
- **Commercial** - Contact kunci115 for commercial license
- **Contributions** - Accepted under AGPL-3.0 terms

### Contributor Rights
- **Credit** - Your name/contributions will be acknowledged
- **Portfolio** - Build your portfolio with real-world projects
- **Community** - Join the distributed version control revolution

## 📞 Getting Help

### 💬 Community
- **GitHub Discussions** - Ask questions and share ideas
- **GitHub Issues** - Report bugs and request features
- **Discord/Slack** - Real-time collaboration (coming soon)

### 📧 Resources
- **API Documentation** - `cargo doc --open`
- **Examples** - `examples/` directory in repository
- **Architecture Guide** - `docs/architecture.md`
- **Performance Benchmarks** - `docs/benchmarks.md`

## 🎉 Recognition

### Contribution Types
We value all types of contributions:

- **💻 Code** - Core functionality, bug fixes, performance improvements
- **📖 Documentation** - README improvements, API docs, tutorials
- **🐛 Bug Reports** - Detailed issue reports with reproduction steps
- **💡 Feature Requests** - Well-described enhancement proposals
- **🧪 Testing** - Test cases, performance benchmarks, CI/CD
- **🎨 Design** - UI/UX improvements, icons, diagrams
- **📢 Translation** - Help translate documentation and interfaces

### Major Contributors
Top contributors will be featured in:
- **README.md** - Hall of fame section
- **Releases** - Acknowledged in release notes
- **Website** - Contributor profiles and interviews

## 🚀 Submitting Changes

### Pull Request Process
1. **Create PR** with clear title and description
2. **Link issues** - Reference any related GitHub issues
3. **Tests passing** - Ensure CI checks pass
4. **Code review** - Respond to reviewer feedback promptly
5. **Merge** - Once approved, maintainers will merge

### Code Review Guidelines
When reviewing others' contributions:
- **Be constructive** - Helpful, respectful feedback
- **Check standards** - Ensure code follows project guidelines
- **Test thoroughly** - Verify changes work as expected
- **Security focus** - Pay attention to security implications

## 📄 Legal

By contributing, you agree that:
- Your contributions will be licensed under **AGPL-3.0**
- You have proper rights to contribute the code
- Contributions follow project standards and guidelines

---

**🎯 Ready to contribute?**

Choose an area that interests you and start building! Every contribution helps make FAI Protocol better for the entire community.

**Questions?** → Check [GitHub Issues](https://github.com/kunci115/fai-protocol/issues) or [Discussions](https://github.com/kunci115/fai-protocol/discussions)

**Thank you for helping build the future of distributed version control!** 🚀