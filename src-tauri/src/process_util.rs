use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn silent_command(program: &str) -> Command {
    let mut command = Command::new(program);
    apply_silent(&mut command);
    command
}

pub fn apply_silent(command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_command_for_program() {
        let command = silent_command("git");
        assert_eq!(command.get_program(), "git");
    }
}
