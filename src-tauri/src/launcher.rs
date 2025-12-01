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
        
        match category {
            TagCategory::Cli | TagCategory::Startup => {
                // For CLI and Startup scripts, open a new CMD window
                // cmd /C start "Title" /D "path" exe args...
                let mut cmd = Command::new("cmd");
                cmd.arg("/C");
                cmd.arg("start");
                cmd.arg(format!("VibeCoding - {}", executable)); // Title
                cmd.arg("/D");
                cmd.arg(project_path);
                cmd.arg(executable);
                
                if let Some(args) = &config.args {
                    for arg in args {
                        cmd.arg(arg);
                    }
                }
                
                // For CLI tools, we don't automatically append project path as arg unless specified
                // But environment variables need to be set? 
                // cmd /C start inherits env? Yes.
                // But we can't easily set per-command env vars with `start` unless we wrap in a block.
                // For now, let's assume global env is enough or user puts env in args.
                
                let child = cmd.spawn()?;
                Ok(child.id() > 0)
            },
            _ => {
                // For IDEs and others, launch directly
                let mut cmd = Command::new(executable);
                
                if let Some(args) = &config.args {
                    for arg in args {
                        cmd.arg(arg);
                    }
                }
                
                // For IDEs, append project path if not explicitly disabled (we assume enabled for now)
                if matches!(category, TagCategory::Ide) {
                    cmd.arg(project_path);
                }
                
                if let Some(env) = &config.env {
                    for (key, value) in env {
                        cmd.env(key, value);
                    }
                }
                
                cmd.current_dir(project_path);
                
                // Detach process
                // use std::os::windows::process::CommandExt;
                // const DETACHED_PROCESS: u32 = 0x00000008;
                // cmd.creation_flags(DETACHED_PROCESS);
                
                let child = cmd.spawn()?;
                Ok(child.id() > 0)
            }
        }
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
