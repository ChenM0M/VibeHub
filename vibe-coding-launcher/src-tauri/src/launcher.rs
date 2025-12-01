use crate::models::{LaunchConfig, ToolType};
use anyhow::{anyhow, Result};
use std::process::Command;

pub struct Launcher;

impl Launcher {
    pub fn launch(
        config: &LaunchConfig,
        project_path: &str,
    ) -> Result<()> {
        #[cfg(target_os = "windows")]
        let status = Self::launch_windows(config, project_path)?;
        
        #[cfg(target_os = "macos")]
        let status = Self::launch_macos(config, project_path)?;
        
        #[cfg(target_os = "linux")]
        let status = Self::launch_linux(config, project_path)?;

        if status {
            Ok(())
        } else {
            Err(anyhow!("Failed to launch tool"))
        }
    }

    #[cfg(target_os = "windows")]
    fn launch_windows(config: &LaunchConfig, project_path: &str) -> Result<bool> {
        let mut cmd = Command::new(&config.executable_path);
        
        // Add arguments
        for arg in &config.arguments {
            cmd.arg(arg);
        }
        
        // Add project path
        cmd.arg(project_path);
        
        // Set environment variables
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }
        
        // Set working directory
        cmd.current_dir(project_path);
        
        // Spawn without waiting (detached process)
        let child = cmd.spawn()?;
        
        Ok(child.id() > 0)
    }

    #[cfg(target_os = "macos")]
    fn launch_macos(config: &LaunchConfig, project_path: &str) -> Result<bool> {
        // For macOS apps, use 'open' command
        let mut cmd = if config.executable_path.ends_with(".app") {
            let mut c = Command::new("open");
            c.arg("-a").arg(&config.executable_path);
            c
        } else {
            Command::new(&config.executable_path)
        };
        
        for arg in &config.arguments {
            cmd.arg(arg);
        }
        
        cmd.arg(project_path);
        
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }
        
        let child = cmd.spawn()?;
        Ok(child.id() > 0)
    }

    #[cfg(target_os = "linux")]
    fn launch_linux(config: &LaunchConfig, project_path: &str) -> Result<bool> {
        let mut cmd = Command::new(&config.executable_path);
        
        for arg in &config.arguments {
            cmd.arg(arg);
        }
        
        cmd.arg(project_path);
        
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }
        
        cmd.current_dir(project_path);
        
        let child = cmd.spawn()?;
        Ok(child.id() > 0)
    }

    pub fn open_in_file_manager(path: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .arg(path)
                .spawn()?;
        }
        
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(path)
                .spawn()?;
        }
        
        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open")
                .arg(path)
                .spawn()?;
        }
        
        Ok(())
    }

    pub fn open_terminal(path: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            // Try Windows Terminal first, fallback to cmd
            let wt_result = Command::new("wt")
                .arg("-d")
                .arg(path)
                .spawn();
            
            if wt_result.is_err() {
                Command::new("cmd")
                    .arg("/c")
                    .arg("start")
                    .arg("cmd")
                    .arg("/k")
                    .arg(format!("cd /d {}", path))
                    .spawn()?;
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg("-a")
                .arg("Terminal")
                .arg(path)
                .spawn()?;
        }
        
        #[cfg(target_os = "linux")]
        {
            // Try common terminal emulators
            let terminals = ["gnome-terminal", "konsole", "xterm"];
            let mut success = false;
            
            for terminal in &terminals {
                if Command::new(terminal)
                    .arg("--working-directory")
                    .arg(path)
                    .spawn()
                    .is_ok()
                {
                    success = true;
                    break;
                }
            }
            
            if !success {
                return Err(anyhow!("No terminal emulator found"));
            }
        }
        
        Ok(())
    }

    pub fn get_default_configs() -> Vec<LaunchConfig> {
        vec![
            LaunchConfig {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Visual Studio Code".to_string(),
                tool_type: ToolType::Vscode,
                executable_path: Self::get_default_vscode_path(),
                arguments: vec![],
                env_vars: vec![],
            },
            LaunchConfig {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Terminal".to_string(),
                tool_type: ToolType::Terminal,
                executable_path: "terminal".to_string(), // Special handler
                arguments: vec![],
                env_vars: vec![],
            },
        ]
    }

    #[cfg(target_os = "windows")]
    fn get_default_vscode_path() -> String {
        "code".to_string() // VSCode usually in PATH
    }

    #[cfg(target_os = "macos")]
    fn get_default_vscode_path() -> String {
        "/Applications/Visual Studio Code.app".to_string()
    }

    #[cfg(target_os = "linux")]
    fn get_default_vscode_path() -> String {
        "code".to_string()
    }
}
