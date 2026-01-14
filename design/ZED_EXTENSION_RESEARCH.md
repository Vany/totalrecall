# Zed Extension Research: MCP Server Integration Patterns

**Date:** 2026-01-14  
**Project:** Total Recall MCP Server  
**Objective:** Understand how to properly integrate MCP servers into Zed editor extensions for cross-platform distribution

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Zed Extension Architecture](#zed-extension-architecture)
3. [MCP Server Extension Pattern](#mcp-server-extension-pattern)
4. [Binary Distribution Strategy](#binary-distribution-strategy)
5. [Reference Implementation Analysis](#reference-implementation-analysis)
6. [Implementation Requirements](#implementation-requirements)
7. [Submission Process](#submission-process)
8. [Sources](#sources)

---

## Executive Summary

Zed editor uses WebAssembly-based extensions to integrate MCP (Model Context Protocol) servers. The recommended pattern for distributing MCP servers with native binaries is:

1. **Extension as WebAssembly**: The extension itself is a Rust library compiled to WASM component format
2. **Binary via GitHub Releases**: The MCP server binary is distributed through GitHub Releases with platform-specific builds
3. **Runtime Download**: The extension downloads the appropriate binary for the user's platform on first use
4. **Version Management**: Extensions cache binaries and automatically clean up old versions

This approach allows a single extension codebase to support macOS (ARM64 + x86_64), Linux (x86_64 + ARM64), and Windows (x86_64) without bundling large binaries in the extension itself.

---

## Zed Extension Architecture

### Directory Structure

A typical Zed MCP extension follows this structure:

```
my-mcp-extension/
├── extension.toml          # Extension manifest
├── Cargo.toml              # Rust library configuration
├── Cargo.lock              # Dependency lock file
├── LICENSE                 # Open source license
├── README.md               # User documentation
├── configuration/          # Configuration templates
│   ├── installation_instructions.md
│   └── default_settings.jsonc
└── src/
    └── lib.rs              # Extension implementation
```

### Required Files

#### extension.toml

Manifest file defining extension metadata:

```toml
id = "my-mcp-server"
name = "My MCP Server"
description = "Description of what the MCP server does"
version = "0.1.0"
schema_version = 1
authors = ["Your Name <email@example.com>"]
repository = "https://github.com/username/zed-my-mcp-server"

[context_servers.my-mcp-server]
name = "My MCP Server"
```

**Key fields:**
- `id`: Unique identifier (kebab-case)
- `schema_version`: Currently `1`
- `[context_servers.<name>]`: Declares the MCP server

#### Cargo.toml

Rust library configuration:

```toml
[package]
name = "my-mcp-server"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
zed_extension_api = "0.7.0"
serde = "1.0"
schemars = "0.8"
```

**Critical requirements:**
- `crate-type = ["cdylib"]` - Required for WASM compilation
- `zed_extension_api = "0.7.0"` - Current API version (as of Jan 2026)
- `serde` and `schemars` - Required dependencies for the extension API

### Build Process

Extensions are compiled to WebAssembly Component format (version 0x1000d), not standard WASM modules:

```bash
# Install target
rustup target add wasm32-unknown-unknown

# Build WASM module
cargo build --release --target wasm32-unknown-unknown

# Convert to Component format
wasm-tools component new \
  target/wasm32-unknown-unknown/release/my_extension.wasm \
  -o extension.wasm
```

**Important**: Zed requires WebAssembly Components (version 0x1000d), not regular WASM modules (version 0x1). Use `wasm-tools component new` to convert.

---

## MCP Server Extension Pattern

### Extension Implementation

Extensions implement the `zed_extension_api::Extension` trait with specific methods for MCP servers:

```rust
use zed_extension_api::{self as zed, Command, ContextServerId, Project, Result};

struct MyMcpExtension {
    cached_binary_path: Option<String>,
}

impl zed::Extension for MyMcpExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn context_server_command(
        &mut self,
        context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<Command> {
        Ok(Command {
            command: self.context_server_binary_path(context_server_id)?,
            args: vec!["serve".to_string()],
            env: vec![],
        })
    }
}

zed::register_extension!(MyMcpExtension);
```

### Key Methods

1. **`context_server_command`** (Required)
   - Returns the command to start the MCP server
   - Must return `Command` with path, args, and environment variables
   - Called when Zed needs to spawn the MCP server

2. **`context_server_configuration`** (Optional)
   - Provides user-facing configuration UI
   - Returns installation instructions and settings schema
   - Enables in-editor configuration

---

## Binary Distribution Strategy

### The GitHub Releases Pattern

Modern Zed MCP extensions use GitHub Releases for cross-platform binary distribution:

**Advantages:**
- No bundling of large binaries in the extension
- Automatic platform detection
- Version management with caching
- Easy updates through new releases

### Platform-Specific Binary Naming

Follow this naming convention for release assets:

```
<binary-name>_<OS>_<ARCH>.<ext>

Examples:
- rag-mcp_Darwin_arm64.tar.gz      # macOS ARM64
- rag-mcp_Darwin_x86_64.tar.gz     # macOS Intel
- rag-mcp_Linux_x86_64.tar.gz      # Linux x86_64
- rag-mcp_Linux_arm64.tar.gz       # Linux ARM64
- rag-mcp_Windows_x86_64.zip       # Windows x86_64
```

**Supported platforms:**
- macOS: `Darwin` (arm64 only - Apple Silicon)
- Linux: `Linux` (x86_64, arm64)
- Windows: `Windows` (x86_64)

**Archive formats:**
- macOS/Linux: `.tar.gz` (gzip-compressed tar)
- Windows: `.zip`

---

## Reference Implementation Analysis

### GitHub MCP Server Extension

**Repository:** [LoamStudios/zed-mcp-server-github](https://github.com/LoamStudios/zed-mcp-server-github)

This extension demonstrates the complete pattern for binary distribution:

```rust
const REPO_NAME: &str = "github/github-mcp-server";
const BINARY_NAME: &str = "github-mcp-server";

impl MyExtension {
    fn context_server_binary_path(
        &mut self,
        _context_server_id: &ContextServerId,
    ) -> Result<String> {
        // Check cache first
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        // Get latest release from GitHub
        let release = zed::latest_github_release(
            REPO_NAME,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        // Determine platform and architecture
        let (platform, arch) = zed::current_platform();
        
        // Build asset name
        let asset_name = format!(
            "{BINARY_NAME}_{os}_{arch}.{ext}",
            arch = match arch {
                zed::Architecture::Aarch64 => "arm64",
                zed::Architecture::X86 => "i386",
                zed::Architecture::X8664 => "x86_64",
            },
            os = match platform {
                zed::Os::Mac => "Darwin",
                zed::Os::Linux => "Linux",
                zed::Os::Windows => "Windows",
            },
            ext = match platform {
                zed::Os::Mac | zed::Os::Linux => "tar.gz",
                zed::Os::Windows => "zip",
            }
        );

        // Find matching asset
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        // Create version directory
        let version_dir = format!("{BINARY_NAME}-{}", release.version);
        fs::create_dir_all(&version_dir)
            .map_err(|err| format!("failed to create directory '{version_dir}': {err}"))?;
        
        let binary_path = format!(
            "{version_dir}/{BINARY_NAME}{suffix}",
            suffix = match platform {
                zed::Os::Windows => ".exe",
                _ => "",
            }
        );

        // Download if not exists
        if !fs::metadata(&binary_path).map_or(false, |stat| stat.is_file()) {
            let file_kind = match platform {
                zed::Os::Mac | zed::Os::Linux => zed::DownloadedFileType::GzipTar,
                zed::Os::Windows => zed::DownloadedFileType::Zip,
            };

            zed::download_file(&asset.download_url, &version_dir, file_kind)
                .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            // Clean up old versions
            let entries = fs::read_dir(".")
                .map_err(|e| format!("failed to list working directory {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        // Cache the path
        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}
```

### Key Insights

1. **Lazy Download**: Binary is downloaded only when first needed
2. **Platform Detection**: Uses `zed::current_platform()` for automatic platform detection
3. **Version Caching**: Stores binary in version-specific directory
4. **Automatic Cleanup**: Removes old versions after successful download
5. **Error Handling**: Comprehensive error messages for debugging

---

## Implementation Requirements

### 1. GitHub Releases Setup

Create releases with platform-specific binaries:

```bash
# Example release: v0.1.0
# Required assets:
- rag-mcp_Darwin_arm64.tar.gz
- rag-mcp_Darwin_x86_64.tar.gz
- rag-mcp_Linux_x86_64.tar.gz
- rag-mcp_Linux_arm64.tar.gz
- rag-mcp_Windows_x86_64.zip
```

### 2. GitHub Actions for Cross-Platform Builds

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
            archive: tar.gz
            asset_name: rag-mcp_Darwin_arm64.tar.gz
          - os: macos-latest
            target: x86_64-apple-darwin
            archive: tar.gz
            asset_name: rag-mcp_Darwin_x86_64.tar.gz
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            archive: tar.gz
            asset_name: rag-mcp_Linux_x86_64.tar.gz
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            archive: tar.gz
            asset_name: rag-mcp_Linux_arm64.tar.gz
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            archive: zip
            asset_name: rag-mcp_Windows_x86_64.zip

    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}
          override: true
      
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Create archive
        run: |
          # Archive creation logic based on platform
          
      - name: Upload Release Asset
        uses: actions/upload-release-asset@v1
        with:
          upload_url: ${{ steps.create_release.outputs.upload_url }}
          asset_path: ./${{ matrix.asset_name }}
          asset_name: ${{ matrix.asset_name }}
          asset_content_type: application/octet-stream
```

### 3. Extension Code Structure

```rust
use schemars::JsonSchema;
use serde::Deserialize;
use std::fs;
use zed_extension_api::{self as zed, Command, ContextServerId, Project, Result};

const REPO_NAME: &str = "username/totalrecall";
const BINARY_NAME: &str = "rag-mcp";

struct TotalRecallExtension {
    cached_binary_path: Option<String>,
}

impl zed::Extension for TotalRecallExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn context_server_command(
        &mut self,
        context_server_id: &ContextServerId,
        _project: &Project,
    ) -> Result<Command> {
        Ok(Command {
            command: self.context_server_binary_path(context_server_id)?,
            args: vec!["serve".to_string()],
            env: vec![],
        })
    }
}

zed::register_extension!(TotalRecallExtension);
```

---

## Submission Process

### 1. Repository Setup

Create a separate repository for the Zed extension:

```
Repository name: zed-totalrecall
Location: Your GitHub account (NOT organization)
```

**Required files:**
- `extension.toml` - Extension manifest
- `Cargo.toml` - Rust configuration
- `src/lib.rs` - Extension implementation
- `LICENSE` - Open source license (MIT, Apache-2.0, etc.)
- `README.md` - User documentation
- `configuration/` - Optional configuration files

### 2. Fork and Submit

1. **Fork** [zed-industries/extensions](https://github.com/zed-industries/extensions)
2. **Add submodule** using HTTPS URL:
   ```bash
   cd extensions
   git submodule add https://github.com/username/zed-totalrecall.git extensions/totalrecall
   ```

3. **Update** `extensions.toml`:
   ```toml
   [totalrecall]
   submodule = "extensions/totalrecall"
   version = "0.1.0"
   ```

4. **Create PR** to zed-industries/extensions

### 3. License Requirements

**Required**: Extension code must use an approved open-source license:
- MIT
- Apache-2.0
- BSD 3-Clause
- GPLv3
- LGPLv3
- zlib

**Note**: License requirement applies only to extension code, not to the MCP server binary.

---

## Best Practices

### Extension Development

1. **Test locally first**: Install as dev extension in Zed
2. **Handle errors gracefully**: Provide clear error messages
3. **Cache binary path**: Avoid repeated downloads
4. **Clean up old versions**: Remove outdated binaries automatically
5. **Validate settings**: Check required configuration before starting server

### Binary Distribution

1. **Version consistently**: Match extension version with binary release
2. **Test all platforms**: Ensure binaries work on all supported platforms
3. **Document requirements**: Specify any system dependencies in README
4. **Provide fallback**: Consider allowing users to specify custom binary path

### Documentation

1. **Installation instructions**: Clear steps for users
2. **Configuration examples**: Show how to configure the MCP server
3. **Troubleshooting**: Common issues and solutions
4. **API documentation**: If MCP server has specific tools/resources

---

## Common Pitfalls

### WASM Compilation Issues

**Problem**: Extension fails to load with "failed to parse WebAssembly module"  
**Solution**: Ensure using Component format (0x1000d), not module format (0x1):
```bash
wasm-tools component new module.wasm -o extension.wasm
```

### API Version Mismatch

**Problem**: Extension panics during initialization  
**Solution**: Match `zed_extension_api` version in `Cargo.toml` with `lib.version` in `extension.toml`:
```toml
# Cargo.toml
zed_extension_api = "0.7.0"

# extension.toml
[lib]
kind = "Rust"
version = "0.7.0"
```

### Missing Dependencies

**Problem**: Extension initialization fails  
**Solution**: Include required dependencies even if not directly used:
```toml
[dependencies]
zed_extension_api = "0.7.0"
serde = "1.0"           # Required
schemars = "0.8"        # Required
```

### Binary Not Executable

**Problem**: Downloaded binary cannot execute (Permission denied)  
**Solution**: Call `zed::make_file_executable()` after download:
```rust
zed::make_file_executable(&binary_path)?;
```

### Platform Detection Issues

**Problem**: Wrong binary downloaded for user's platform  
**Solution**: Use `zed::current_platform()` instead of manual detection:
```rust
let (platform, arch) = zed::current_platform();
```

---

## Conclusion

Zed's extension system for MCP servers provides a clean separation between extension code (WASM) and server binaries (platform-specific). The GitHub Releases pattern enables:

- ✅ Single extension codebase for all platforms
- ✅ Automatic platform detection
- ✅ On-demand binary downloads
- ✅ Version management and caching
- ✅ Easy updates through new releases

This research provides a complete blueprint for integrating the Total Recall MCP server into Zed's extension ecosystem with proper cross-platform support.

---

## Sources

1. [Zed Extensions Repository](https://github.com/zed-industries/extensions)
2. [Zed Extension Development Documentation](https://zed.dev/docs/extensions/developing-extensions)
3. [MCP Extensions Documentation](https://zed.dev/docs/extensions/mcp-extensions)
4. [GitHub MCP Server Extension](https://github.com/LoamStudios/zed-mcp-server-github) - Reference implementation
5. [Postgres Context Server](https://github.com/zed-extensions/postgres-context-server) - Reference implementation
6. [Zed Extension API (zed_extension_api)](https://docs.rs/zed_extension_api/latest/zed_extension_api/)

---

**Document Version:** 1.0  
**Last Updated:** 2026-01-14  
**Author:** Research for Total Recall MCP Server Integration
