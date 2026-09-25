use std::io;
use std::process::{Command, ExitStatus};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub fn execute_command_line(cmdline: &str) -> io::Result<ExitStatus> {
  #[cfg(target_os = "windows")]
  {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(cmdline);
    cmd.creation_flags(0x08000000);
    cmd.status()
  }

  #[cfg(not(target_os = "windows"))]
  {
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c").arg(cmdline);
    cmd.status()
  }
}
