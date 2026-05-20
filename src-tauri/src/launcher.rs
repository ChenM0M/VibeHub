use crate::models::{Project, TagCategory, TagConfig};
#[cfg(target_os = "windows")]
use crate::process_util::silent_command;
use anyhow::{anyhow, Result};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Command;
#[cfg(target_os = "macos")]
use std::{fs, os::unix::fs::PermissionsExt, thread, time::Duration};

pub struct Launcher;

impl Launcher {
    pub fn launch(project: &Project, configs: &[(TagConfig, TagCategory)]) -> Result<()> {
        let mut success = false;

        for (config, category) in configs {
            if let Some(executable) = &config.executable {
                #[cfg(target_os = "windows")]
                if Self::launch_windows(executable, config, category, &project.path)? {
                    success = true;
                }

                #[cfg(target_os = "macos")]
                if Self::launch_macos(executable, config, category, &project.path)? {
                    success = true;
                }

                #[cfg(target_os = "linux")]
                if Self::launch_linux(executable, config, category, &project.path)? {
                    success = true;
                }
            }
        }

        if success {
            Ok(())
        } else if configs.is_empty() {
            Err(anyhow!("No launch configuration found for the selected tags. Please configure the tags or use custom launch."))
        } else {
            Err(anyhow!("Failed to launch any tools"))
        }
    }

    #[cfg(target_os = "windows")]
    fn launch_windows(
        executable: &str,
        config: &TagConfig,
        category: &TagCategory,
        project_path: &str,
    ) -> Result<bool> {
        println!(
            "Launching on Windows: exe={}, path={}, category={:?}",
            executable, project_path, category
        );

        if matches!(category, TagCategory::Cli) {
            return Self::launch_windows_cli(executable, config, project_path);
        }

        // Unified non-CLI launch strategy using `cmd /C start`. This keeps batch
        // files such as code.cmd working and lets GUI apps detach cleanly.
        Self::launch_windows_cmd(executable, config, category, project_path)
    }

    #[cfg(target_os = "windows")]
    fn launch_windows_cmd(
        executable: &str,
        config: &TagConfig,
        category: &TagCategory,
        project_path: &str,
    ) -> Result<bool> {
        let mut cmd = silent_command("cmd");
        cmd.arg("/C");
        cmd.arg("start");
        cmd.arg(format!("VibeHub - {}", executable)); // Title (first quoted arg)
        cmd.arg("/D");
        cmd.arg(project_path); // Working directory

        // The executable to run
        cmd.arg(executable);

        // User arguments
        if let Some(args) = &config.args {
            for arg in args {
                cmd.arg(arg);
            }
        }

        // For IDEs, append project path as an argument
        if matches!(category, TagCategory::Ide) {
            cmd.arg(project_path);
        }

        // Apply environment variables to the cmd process
        // The started process inherits these
        if let Some(env) = &config.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        println!("Executing command: {:?}", cmd);

        let child = cmd.spawn()?;
        Ok(child.id() > 0)
    }

    #[cfg(target_os = "windows")]
    fn launch_windows_cli(
        executable: &str,
        config: &TagConfig,
        project_path: &str,
    ) -> Result<bool> {
        match Self::normalize_windows_terminal(config.terminal.as_deref()).as_str() {
            "powershell" => Self::launch_windows_powershell(executable, config, project_path),
            "wt" => Self::launch_windows_terminal(executable, config, project_path),
            _ => Self::launch_windows_cmd(executable, config, &TagCategory::Cli, project_path),
        }
    }

    #[cfg(target_os = "windows")]
    fn launch_windows_terminal(
        executable: &str,
        config: &TagConfig,
        project_path: &str,
    ) -> Result<bool> {
        let mut cmd = silent_command("wt.exe");
        cmd.arg("-d").arg(project_path).arg(executable);

        if let Some(args) = &config.args {
            for arg in args {
                cmd.arg(arg);
            }
        }

        if let Some(env) = &config.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        cmd.current_dir(project_path);

        match cmd.spawn() {
            Ok(child) => Ok(child.id() > 0),
            Err(_) => Self::launch_windows_cmd(executable, config, &TagCategory::Cli, project_path),
        }
    }

    #[cfg(target_os = "windows")]
    fn launch_windows_powershell(
        executable: &str,
        config: &TagConfig,
        project_path: &str,
    ) -> Result<bool> {
        let mut shell_command = format!(
            "Set-Location -LiteralPath {}; & {}",
            Self::powershell_quote(project_path),
            Self::powershell_quote(executable)
        );

        if let Some(args) = &config.args {
            for arg in args {
                shell_command.push(' ');
                shell_command.push_str(&Self::powershell_quote(arg));
            }
        }

        let mut cmd = silent_command("cmd");
        cmd.arg("/C");
        cmd.arg("start");
        cmd.arg(format!("VibeHub - {}", executable));
        cmd.arg("/D");
        cmd.arg(project_path);
        cmd.arg("powershell.exe");
        cmd.arg("-NoExit");
        cmd.arg("-ExecutionPolicy");
        cmd.arg("Bypass");
        cmd.arg("-Command");
        cmd.arg(shell_command);

        if let Some(env) = &config.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        let child = cmd.spawn()?;
        Ok(child.id() > 0)
    }

    #[cfg(target_os = "windows")]
    fn normalize_windows_terminal(terminal: Option<&str>) -> String {
        match terminal.unwrap_or("").trim().to_ascii_lowercase().as_str() {
            "windows terminal" | "windowsterminal" | "wt" | "wt.exe" => "wt".to_string(),
            "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" => "powershell".to_string(),
            "command prompt" | "commandprompt" | "cmd" | "cmd.exe" => "cmd".to_string(),
            _ => "cmd".to_string(),
        }
    }

    #[cfg(target_os = "windows")]
    fn powershell_quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "''"))
    }

    #[cfg(target_os = "macos")]
    fn launch_macos(
        executable: &str,
        config: &TagConfig,
        category: &TagCategory,
        project_path: &str,
    ) -> Result<bool> {
        let shell_command = Self::build_shell_command(executable, config, category, project_path)?;

        if matches!(category, TagCategory::Cli) {
            return Self::launch_macos_terminal(
                config.terminal.as_deref().unwrap_or("Terminal"),
                &shell_command,
            );
        }

        if executable.ends_with(".app") {
            return Self::launch_macos_app(executable, config, category, project_path);
        }

        let child = {
            let mut cmd = Command::new("/bin/zsh");
            cmd.arg("-lc").arg(shell_command);
            cmd.current_dir(project_path);
            if let Some(env) = &config.env {
                for (key, value) in env {
                    cmd.env(key, value);
                }
            }
            cmd.spawn()?
        };

        Ok(child.id() > 0)
    }

    #[cfg(target_os = "macos")]
    fn launch_macos_terminal(terminal: &str, shell_command: &str) -> Result<bool> {
        let normalized = Self::macos_app_name(terminal).to_lowercase();
        match normalized.as_str() {
            "warp" => Self::launch_warp_command(shell_command),
            "iterm" | "iterm2" => Self::launch_iterm_command(shell_command),
            _ => Self::launch_terminal_command(shell_command),
        }
    }

    #[cfg(target_os = "macos")]
    fn launch_terminal_command(shell_command: &str) -> Result<bool> {
        let script = format!(
            "tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell",
            Self::escape_applescript_string(shell_command)
        );
        let child = Command::new("osascript").arg("-e").arg(script).spawn()?;
        Ok(child.id() > 0)
    }

    #[cfg(target_os = "macos")]
    fn launch_iterm_command(shell_command: &str) -> Result<bool> {
        let script = format!(
            "tell application \"iTerm\"\nactivate\ncreate window with default profile\ntell current session of current window\nwrite text \"{}\"\nend tell\nend tell",
            Self::escape_applescript_string(shell_command)
        );
        let child = Command::new("osascript").arg("-e").arg(script).spawn()?;
        Ok(child.id() > 0)
    }

    #[cfg(target_os = "macos")]
    fn launch_warp_command(shell_command: &str) -> Result<bool> {
        let script_dir = std::env::temp_dir().join("vibehub-launch");
        fs::create_dir_all(&script_dir)?;

        let script_path = script_dir.join(format!("{}.command", uuid::Uuid::new_v4()));
        fs::write(&script_path, format!("#!/bin/zsh\n{}\n", shell_command))?;

        let mut permissions = fs::metadata(&script_path)?.permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script_path, permissions)?;

        let cleanup_path = script_path.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(120));
            let _ = fs::remove_file(cleanup_path);
        });

        let child = Command::new("open")
            .arg("-a")
            .arg("Warp")
            .arg(script_path)
            .spawn()?;
        Ok(child.id() > 0)
    }

    #[cfg(target_os = "macos")]
    fn launch_macos_app(
        executable: &str,
        config: &TagConfig,
        category: &TagCategory,
        project_path: &str,
    ) -> Result<bool> {
        let mut cmd = Command::new("open");
        cmd.arg("-a").arg(Self::macos_app_name(executable));

        if matches!(category, TagCategory::Ide) {
            cmd.arg(project_path);
        }

        if let Some(args) = &config.args {
            if !args.is_empty() {
                cmd.arg("--args");
                for arg in args {
                    cmd.arg(arg);
                }
            }
        }

        let child = cmd.spawn()?;
        Ok(child.id() > 0)
    }

    #[cfg(target_os = "macos")]
    fn macos_app_name(executable: &str) -> String {
        std::path::Path::new(executable)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or(executable)
            .to_string()
    }

    #[cfg(target_os = "macos")]
    fn build_shell_command(
        executable: &str,
        config: &TagConfig,
        category: &TagCategory,
        project_path: &str,
    ) -> Result<String> {
        let mut command_parts = Vec::new();

        if let Some(env) = &config.env {
            for (key, value) in env {
                Self::validate_env_key(key)?;
                command_parts.push(format!("{}={}", key, Self::shell_quote(value)));
            }
        }

        command_parts.push(Self::shell_quote(executable));

        if let Some(args) = &config.args {
            for arg in args {
                command_parts.push(Self::shell_quote(arg));
            }
        }

        if matches!(category, TagCategory::Ide) {
            command_parts.push(Self::shell_quote(project_path));
        }

        Ok(format!(
            "cd {} && {}",
            Self::shell_quote(project_path),
            command_parts.join(" ")
        ))
    }

    #[cfg(target_os = "macos")]
    fn validate_env_key(key: &str) -> Result<()> {
        let mut chars = key.chars();
        let Some(first) = chars.next() else {
            return Err(anyhow!("Environment variable name cannot be empty"));
        };

        if !(first == '_' || first.is_ascii_alphabetic())
            || !chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
        {
            return Err(anyhow!("Invalid environment variable name: {}", key));
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn shell_quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\\''"))
    }

    #[cfg(target_os = "macos")]
    fn escape_applescript_string(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', " ")
    }

    #[cfg(target_os = "linux")]
    fn launch_linux(
        executable: &str,
        config: &TagConfig,
        category: &TagCategory,
        project_path: &str,
    ) -> Result<bool> {
        // Linux implementation
        let mut cmd = Command::new(executable);

        if matches!(category, TagCategory::Cli) {
            // Try to launch in terminal
            // This is complex on Linux due to many terminal emulators
            // For now, just run directly
        }

        if let Some(args) = &config.args {
            for arg in args {
                cmd.arg(arg);
            }
        }

        if matches!(category, TagCategory::Ide) {
            cmd.arg(project_path);
        }

        if let Some(env) = &config.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        cmd.current_dir(project_path);

        let child = cmd.spawn()?;
        Ok(child.id() > 0)
    }
}
