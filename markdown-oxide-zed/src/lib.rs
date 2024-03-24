use zed::Command;
use zed_extension_api as zed;

struct Moxide {

}


impl zed::Extension for Moxide {
    fn language_server_command(
            &mut self,
            config: zed::LanguageServerConfig,
            worktree: &zed::Worktree,
        ) -> zed::Result<zed::Command> {


        zed::set_language_server_installation_status(
            &config.name,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        Ok(Command {
            command: "markdown-oxide".to_string(),
            args: Default::default(),
            env: Default::default()
        })

    }

    fn new() -> Self
        where
            Self: Sized {
        
        Moxide{}
    }
}

zed::register_extension!(Moxide);
