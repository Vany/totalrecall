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
        // Use full path to avoid PATH issues
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/vany".to_string());
        let binary_path = format!("{}/.cargo/bin/rag-mcp", home);

        Ok(Command {
            command: binary_path,
            args: vec!["serve".to_string()],
            env: vec![],
        })
    }
}

zed::register_extension!(TotalRecallExtension);
