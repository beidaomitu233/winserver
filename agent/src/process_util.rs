//! Windows-safe process helpers.
//!
//! Console subsystem binaries (mysqld.exe, mysql.exe, nginx.exe, …) will open a
//! visible console when launched from a GUI app unless CREATE_NO_WINDOW is set.
//!
//! Important: do **not** combine CREATE_NO_WINDOW with DETACHED_PROCESS.
//! MSDN: CREATE_NO_WINDOW is ignored when used together with DETACHED_PROCESS.

use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

/// CREATE_NO_WINDOW — run a console app without allocating a console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Apply the Windows flag that suppresses console windows.
pub fn apply_no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

/// Build a `Command` that never flashes a console window on Windows.
pub fn silent_command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    apply_no_window(&mut cmd);
    cmd
}

/// Build a silent `Command` from a filesystem path.
pub fn silent_command_path(exe: &Path) -> Command {
    silent_command(exe.as_os_str())
}
