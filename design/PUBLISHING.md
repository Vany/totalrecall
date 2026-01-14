# Publishing Total Recall to Zed Extensions Marketplace

## Current Status

✅ **Ready for Local Testing**
- Extension structure complete
- MCP server implementation working
- 9 integration tests passing

⏳ **Ready for Official Publication** (with binary distribution)

## Steps to Publish

### Option 1: Quick Test (Dev Extension)

For immediate testing in your local Zed:

1. **Install the binary**:
   ```bash
   cargo install --path crates/rag-mcp-server
   ```

2. **Install as dev extension** in Zed:
   - `Cmd+Shift+P` → "zed: install dev extension"
   - Select the `zed-extension` directory

3. **Test with Claude** - it should work immediately!

### Option 2: Official Publication (Requires Binary Distribution)

To publish to the official Zed extensions marketplace, we need to:

#### 1. Set Up Binary Distribution

We need to provide binaries for all platforms. Options:

**A. GitHub Releases** (Recommended)
```bash
# Build for all platforms
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cargo build --release --target x86_64-pc-windows-msvc

# Create releases on GitHub
# Tag: v0.1.0
# Upload binaries as release artifacts
```

**B. cargo-binstall** (Alternative)
```bash
# Publish to crates.io
cargo publish -p rag-mcp-server

# Users install via:
cargo install rag-mcp
```

#### 2. Update Extension to Download Binary

Modify `zed-extension/src/lib.rs` to download the binary:

```rust
use zed_extension_api::{self as zed, Command, ContextServerId, Project, Result};

struct TotalRecallExtension;

impl zed::Extension for TotalRecallExtension {
    fn new() -> Self {
        Self
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &ContextServerId,
        _project: &Project,
    ) -> Result<Command> {
        // Download binary from GitHub releases if not present
        let binary_path = self.get_or_download_binary()?;
        
        Ok(Command {
            command: binary_path,
            args: vec!["serve".to_string()],
            env: vec![],
        })
    }
}

zed::register_extension!(TotalRecallExtension);
```

#### 3. Submit to Zed Extensions Repository

1. **Fork** https://github.com/zed-industries/extensions

2. **Add as submodule**:
   ```bash
   cd extensions
   git submodule add https://github.com/Vany/totalrecall extensions/totalrecall
   ```

3. **Update `extensions.toml`**:
   ```toml
   [totalrecall]
   path = "extensions/totalrecall/zed-extension"
   version = "0.1.0"
   ```

4. **Run sort**:
   ```bash
   pnpm sort-extensions
   ```

5. **Submit PR** with:
   - Clear description
   - Screenshots/demos
   - Testing instructions

## Prerequisites for Official Publication

Before submitting to the official marketplace:

- [ ] Choose a license (we have MIT OR Apache-2.0 ✓)
- [ ] Set up GitHub releases with binaries for all platforms
- [ ] Update extension to download binaries automatically
- [ ] Test on multiple platforms (Linux, macOS, Windows)
- [ ] Create screenshots/demo
- [ ] Write comprehensive README

## Current Recommendation

**Start with local testing** using the dev extension approach. Once you've validated it works well, we can:

1. Set up automated binary releases (GitHub Actions)
2. Implement binary downloading in the extension
3. Submit to official marketplace

## Why Binary Distribution?

Zed extensions should "just work" - users shouldn't need to:
- Install Rust
- Build from source
- Manually install binaries

The extension should automatically download the appropriate binary for the user's platform.

## Example: Look at Existing Extensions

Check out how other Zed MCP extensions handle binary distribution:
- https://github.com/zed-industries/extensions

Many download binaries from:
- GitHub Releases
- npm packages
- Other package managers

## Next Steps

1. **Test locally first** - verify everything works
2. **Set up CI/CD** - automated binary builds
3. **Implement binary download** - make it seamless
4. **Submit PR** - publish to marketplace

For now, the dev extension approach lets you use Total Recall immediately in Zed!
