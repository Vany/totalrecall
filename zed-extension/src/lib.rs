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
        // For now, assume the binary is in PATH
        // TODO: Download/bundle the binary for official distribution
        Ok(Command {
            command: "rag-mcp".to_string(),
            args: vec!["serve".to_string()],
            env: vec![],
        })
    }
}

zed::register_extension!(TotalRecallExtension);
