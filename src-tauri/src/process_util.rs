use std::process::Command;

pub fn require_existing_path(path: &std::path::Path) -> Result<(), String> {
    match path.try_exists() {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "OPEN_FAILED: path does not exist: {}",
            path.display()
        )),
        Err(error) => Err(format!("OPEN_FAILED: {}: {error}", path.display())),
    }
}

#[cfg(target_os = "windows")]
pub fn open_windows_path(path: &std::path::Path) -> Result<(), String> {
    use windows_sys::Win32::{
        System::Com::{
            CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
        },
        UI::{
            Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };
    require_existing_path(path)?;
    use std::os::windows::ffi::OsStrExt;
    // Simplify only when the path denotes the same file; preserve extended-path semantics.
    let wide: Vec<u16> = dunce::simplified(path)
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // A fresh STA owns COM initialization; no UI-thread or runtime-pool apartment assumptions.
    std::thread::Builder::new()
        .name("native-file-open".into())
        .spawn(move || unsafe {
            let initialized = CoInitializeEx(
                std::ptr::null(),
                (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
            );
            if initialized < 0 {
                return Err(format!("OPEN_FAILED: COM initialization {initialized:#x}"));
            }
            let mut request = SHELLEXECUTEINFOW {
                cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
                lpFile: wide.as_ptr(),
                nShow: SW_SHOWNORMAL,
                ..Default::default()
            };
            let result = if ShellExecuteExW(&mut request) != 0 {
                Ok(())
            } else {
                Err(format!("OPEN_FAILED: {}", std::io::Error::last_os_error()))
            };
            CoUninitialize();
            result
        })
        .map_err(|error| format!("OPEN_FAILED: {error}"))?
        .join()
        .map_err(|_| "OPEN_FAILED: native file-open worker stopped".to_owned())?
}

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn silent_command(program: &str) -> Command {
    let mut command = Command::new(program);
    apply_silent(&mut command);
    command
}

pub fn apply_silent(_command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        _command.creation_flags(CREATE_NO_WINDOW);
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

    #[test]
    fn vanished_target_is_an_error_before_dispatch() {
        let root = std::env::temp_dir().join(format!("vibehub-open-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("file with spaces.txt");
        std::fs::write(&file, "fixture").unwrap();
        assert!(super::require_existing_path(&file).is_ok());
        std::fs::rename(&file, root.join("retained-file.txt")).unwrap();
        assert!(super::require_existing_path(&file)
            .unwrap_err()
            .contains("path does not exist"));
        // Keep this fixture for the validation runner to move to the system Trash.
        eprintln!("retained native-open fixture: {}", root.display());
    }
}
