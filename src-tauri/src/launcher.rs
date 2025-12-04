use crate::models::{Project, TagConfig, TagCategory};
use anyhow::{anyhow, Result};
use std::process::Command;

pub struct Launcher;

impl Launcher {
    pub fn launch(
        project: &Project,
        configs: &[(TagConfig, TagCategory)],
    ) -> Result<()> {
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
    fn launch_windows(executable: &str, config: &TagConfig, category: &TagCategory, project_path: &str) -> Result<bool> {
        println!("Launching on Windows: exe={}, path={}, category={:?}", executable, project_path, category);
        
        // Unified launch strategy using `cmd /C start`
        // This ensures:
        // 1. Environment variables are correctly inherited
        // 2. Batch files (like code.cmd) work as well as .exe
        // 3. GUI apps launch independently
        // 4. CLI apps get their own window
        
        let mut cmd = Command::new("cmd");
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

    #[cfg(target_os = "macos")]
    fn launch_macos(executable: &str, config: &TagConfig, category: &TagCategory, project_path: &str) -> Result<bool> {
        // MacOS implementation (simplified for now, focusing on Windows as requested)
        let mut cmd = if executable.ends_with(".app") {
            let mut c = Command::new("open");
            c.arg("-a").arg(executable);
            c
        } else {
            // For CLI on Mac, we might want to open Terminal
            if matches!(category, TagCategory::Cli) {
                let mut c = Command::new("open");
                c.arg("-a").arg("Terminal");
                c.arg(executable); // This might not work directly, usually needs a script
                c
            } else {
                Command::new(executable)
            }
        };
        
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

    #[cfg(target_os = "linux")]
    fn launch_linux(executable: &str, config: &TagConfig, category: &TagCategory, project_path: &str) -> Result<bool> {
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
