use std::fs;
use zed_extension_api::{self as zed, Command, ContextServerId, Project, Result};

const REPO_NAME: &str = "Vany/totalrecall";
const BINARY_NAME: &str = "rag-mcp";

struct TotalRecallExtension {
    cached_binary_path: Option<String>,
}

impl TotalRecallExtension {
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

        // Build asset name following the pattern: rag-mcp_<OS>_<ARCH>.<ext>
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

        // Create version-specific directory
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
            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
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
