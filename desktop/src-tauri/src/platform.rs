use std::path::Path;
use std::process::Command;

/// 阻止子进程在 Windows 上弹出控制台窗口。
///
/// GUI 应用通过 `std::process::Command` 启动控制台程序（explorer、powershell、
/// aria2c 等）时，Windows 会为子进程创建新的控制台窗口。这里统一设置
/// `CREATE_NO_WINDOW`，使运行期间不出现终端窗口。非 Windows 平台不改变行为。
pub(crate) fn hide_console_window(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

pub(crate) fn open_path_command(path: &Path) -> Command {
    let mut command = if cfg!(target_os = "windows") {
        Command::new("explorer")
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };
    command.arg(path);
    hide_console_window(&mut command);
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
