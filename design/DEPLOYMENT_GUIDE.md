# Total Recall - Zed Extension Deployment Guide

This guide walks through deploying Total Recall to the Zed extensions marketplace.

## Repository Structure

We have:
- **Main repository**: `https://github.com/Vany/totalrecall` - Contains both the MCP server AND the Zed extension
- **Fork**: `https://github.com/Vany/zed-extensions` - Your fork of zed-industries/extensions for submission

## Prerequisites

1. ✅ Main repository with extension code at `zed-extension/`
2. ✅ GitHub Actions workflow for cross-platform builds (`.github/workflows/release.yml`)
3. ✅ Fork of zed-industries/extensions
4. ⏳ First release with binaries

## Step-by-Step Deployment

### Step 1: Create First Release

Create a tagged release to trigger binary builds:

```bash
cd /Users/vany/l/totalrecall

# Ensure all changes are committed
git add .
git commit -m "Prepare for v0.1.0 release"
git push origin main

# Create and push tag
git tag -a v0.1.0 -m "Initial release - BM25 memory system for Zed"
git push origin v0.1.0
```

This will trigger the GitHub Actions workflow which builds binaries for:
- macOS ARM64: `rag-mcp_Darwin_arm64.tar.gz`
- Linux x86_64: `rag-mcp_Linux_x86_64.tar.gz`
- Linux ARM64: `rag-mcp_Linux_arm64.tar.gz`
- Windows x86_64: `rag-mcp_Windows_x86_64.zip`

**Wait for the build to complete** (check Actions tab on GitHub).

### Step 2: Verify Release Assets

Go to `https://github.com/Vany/totalrecall/releases/tag/v0.1.0` and verify:
- All 4 platform binaries are present
- Archives can be downloaded
- Release notes are generated

### Step 3: Test Extension Locally

Before submitting, test the extension works:

```bash
# Build the extension WASM
cd zed-extension
cargo build --release --target wasm32-unknown-unknown

# Convert to Component format
wasm-tools component new \
  target/wasm32-unknown-unknown/release/totalrecall.wasm \
  -o extension.wasm

# Install as dev extension in Zed
# In Zed: Cmd+Shift+P → "zed: install dev extension"
# Select: /Users/vany/l/totalrecall/zed-extension
```

**Test in Zed:**
1. Open Zed
2. Start a Claude Code session
3. Ask: "Store a test memory: Rust is awesome"
4. Ask: "Search for Rust"
5. Verify it works

### Step 4: Add Extension to Your Fork

Clone your fork and add the extension as a submodule:

```bash
# Clone your fork
cd ~/Projects  # or wherever you keep repos
git clone https://github.com/Vany/zed-extensions.git
cd zed-extensions

# Add totalrecall as submodule pointing to the zed-extension/ directory
git submodule add -b main https://github.com/Vany/totalrecall.git extensions/totalrecall

# This creates a submodule pointing to the main repo
# We need to configure it to use the zed-extension/ subdirectory
```

**Important**: We need the submodule to point to the `zed-extension/` directory specifically.

Edit `.gitmodules`:
```ini
[submodule "extensions/totalrecall"]
    path = extensions/totalrecall
    url = https://github.com/Vany/totalrecall.git
```

Actually, **there's a better approach**: Create a separate branch in the main repo that contains ONLY the extension:

```bash
cd /Users/vany/l/totalrecall

# Create orphan branch for extension
git checkout --orphan extension
git rm -rf .
git clean -fdx

# Copy extension files
cp -r zed-extension/* .
cp zed-extension/.gitignore .

# Commit
git add .
git commit -m "Zed extension for Total Recall"
git push origin extension
```

Then add the submodule pointing to this branch:

```bash
cd ~/Projects/zed-extensions
git submodule add -b extension https://github.com/Vany/totalrecall.git extensions/totalrecall
```

### Step 5: Update extensions.toml

Edit `extensions.toml` in your fork and add:

```toml
[totalrecall]
submodule = "extensions/totalrecall"
version = "0.1.0"
```

Keep the file sorted alphabetically.

### Step 6: Commit and Push to Fork

```bash
cd ~/Projects/zed-extensions

git add .gitmodules extensions/totalrecall extensions.toml
git commit -m "Add Total Recall MCP extension"
git push origin main
```

### Step 7: Create Pull Request

1. Go to `https://github.com/Vany/zed-extensions`
2. Click "Contribute" → "Open pull request"
3. Base: `zed-industries/extensions:main`
4. Head: `Vany/zed-extensions:main`
5. Title: "Add Total Recall MCP extension"
6. Description:
   ```markdown
   # Total Recall MCP Extension
   
   Memory system with BM25 search for storing and retrieving context across coding sessions.
   
   ## Features
   - BM25 keyword search
   - Multi-scope storage (session/project/global)
   - SQLite-based persistence
   - Cross-platform support
   
   ## Testing
   - ✅ Tested on macOS ARM64
   - ✅ Extension builds successfully
   - ✅ Binaries available for all platforms
   
   ## Links
   - Repository: https://github.com/Vany/totalrecall
   - License: MIT
   ```

### Step 8: Wait for Review

The Zed team will review your PR. They may ask for:
- Changes to description
- Code improvements
- Testing on other platforms

## Alternative: Simpler Approach

If the submodule approach is complex, we could:

1. Create a separate `zed-totalrecall` repository containing ONLY the extension code
2. Add that as a submodule to zed-extensions

Let me know which approach you prefer!

## Updating the Extension

When you need to update:

```bash
# In main repo
git tag v0.1.1
git push origin v0.1.1

# Wait for binaries to build

# Update version in extension.toml
cd zed-extension
# Edit extension.toml: version = "0.1.1"
git commit -am "Bump version to 0.1.1"
git push origin extension

# In zed-extensions fork
cd ~/Projects/zed-extensions
git submodule update --remote extensions/totalrecall
# Edit extensions.toml: version = "0.1.1"
git commit -am "Update totalrecall to 0.1.1"
git push origin main

# Create new PR
```

## Troubleshooting

### Build Fails on GitHub Actions

Check the Actions tab for errors. Common issues:
- Missing dependencies for cross-compilation
- Incorrect target triple
- Archive creation failures

### Extension Won't Load in Zed

- Verify WASM is Component format: `file extension.wasm` should show version 0x1000d
- Check Zed logs for errors
- Ensure `zed_extension_api` version matches

### Binary Download Fails

- Verify release exists with correct asset names
- Check `REPO_NAME` in `src/lib.rs` matches your GitHub repo
- Ensure assets follow naming convention exactly

## Next Steps

After deployment:
1. Monitor issues on GitHub
2. Respond to user feedback
3. Plan next features (vector embeddings, AST chunking, etc.)
4. Keep binaries updated with bug fixes

---

**Status**: Ready to deploy once you create the v0.1.0 release!
