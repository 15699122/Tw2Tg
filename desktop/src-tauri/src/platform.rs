use std::path::Path;
use std::process::Command;

pub(crate) fn open_path_command(path: &Path) -> Command {
    let mut command = if cfg!(target_os = "windows") {
        Command::new("explorer")
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };
    command.arg(path);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_platform_open_command_with_the_requested_path() {
        let path = Path::new("X-Archive");
        let command = open_path_command(path);
        let args = command
            .get_args()
            .map(|argument| argument.to_owned())
            .collect::<Vec<_>>();
        assert_eq!(args, vec![path.as_os_str()]);
    }
}
