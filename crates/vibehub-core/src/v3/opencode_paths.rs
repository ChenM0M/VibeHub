use super::agent_profile_storage::{
    RuntimePlatform, RuntimeTarget, RuntimeTargetKind, StorageError,
};
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub struct OpenCodeConfigEnvironment {
    pub xdg_config_home: Option<String>,
    pub config_file: Option<String>,
    pub config_directory: Option<String>,
}

fn observed_environment(target: &RuntimeTarget) -> Result<OpenCodeConfigEnvironment, StorageError> {
    if target.kind == RuntimeTargetKind::Wsl {
        #[cfg(windows)]
        {
            let distribution = target.distribution.as_deref().ok_or_else(|| {
                StorageError::new("OPENCODE_ENV_UNAVAILABLE", "missing WSL distribution")
            })?;
            let output = crate::process_util::silent_command("wsl.exe")
                .args(["-d", distribution, "--", "sh", "-lc", "printf '%s\\0%s\\0%s\\0' \"$XDG_CONFIG_HOME\" \"$OPENCODE_CONFIG\" \"$OPENCODE_CONFIG_DIR\""])
                .output().map_err(|error| StorageError::new("OPENCODE_ENV_UNAVAILABLE", error.to_string()))?;
            if !output.status.success() {
                return Err(StorageError::new(
                    "OPENCODE_ENV_UNAVAILABLE",
                    "could not inspect the selected WSL environment",
                ));
            }
            let text = String::from_utf8(output.stdout).map_err(|_| {
                StorageError::new(
                    "OPENCODE_ENV_UNAVAILABLE",
                    "WSL returned invalid environment text",
                )
            })?;
            let values: Vec<_> = text.split('\0').collect();
            if values.len() != 4 {
                return Err(StorageError::new(
                    "OPENCODE_ENV_UNAVAILABLE",
                    "unexpected WSL environment response",
                ));
            }
            return Ok(OpenCodeConfigEnvironment {
                xdg_config_home: nonempty(values[0]),
                config_file: nonempty(values[1]),
                config_directory: nonempty(values[2]),
            });
        }
        #[cfg(not(windows))]
        if env::var("WSL_DISTRO_NAME").ok().as_deref() != target.distribution.as_deref() {
            return Err(StorageError::new(
                "OPENCODE_ENV_UNAVAILABLE",
                "cannot inspect a different WSL distribution from this host",
            ));
        }
    }
    Ok(OpenCodeConfigEnvironment {
        xdg_config_home: env::var("XDG_CONFIG_HOME")
            .ok()
            .and_then(|value| nonempty(&value)),
        config_file: env::var("OPENCODE_CONFIG")
            .ok()
            .and_then(|value| nonempty(&value)),
        config_directory: env::var("OPENCODE_CONFIG_DIR")
            .ok()
            .and_then(|value| nonempty(&value)),
    })
}

fn nonempty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn absolute_for_runtime(target: &RuntimeTarget, value: &str) -> bool {
    if target.platform == RuntimePlatform::Windows {
        value.starts_with(r"\\")
            || (value.as_bytes().get(1) == Some(&b':')
                && matches!(value.as_bytes().get(2), Some(b'\\' | b'/')))
    } else {
        value.starts_with('/')
    }
}

fn runtime_path(target: &RuntimeTarget, value: &str) -> Result<PathBuf, StorageError> {
    if !absolute_for_runtime(target, value) || value.contains('\0') {
        return Err(StorageError::new(
            "OPENCODE_CONFIG_PATH_NOT_ABSOLUTE",
            "use an absolute path in OPENCODE_CONFIG or OPENCODE_CONFIG_DIR",
        ));
    }
    if cfg!(windows) && target.kind == RuntimeTargetKind::Wsl {
        let distribution = target.distribution.as_deref().ok_or_else(|| {
            StorageError::new("OPENCODE_ENV_UNAVAILABLE", "missing WSL distribution")
        })?;
        return Ok(PathBuf::from(format!(
            r"\\wsl$\{distribution}{}",
            value.replace('/', r"\")
        )));
    }
    Ok(PathBuf::from(value))
}

pub fn opencode_config_paths(target: &RuntimeTarget) -> Result<Vec<PathBuf>, StorageError> {
    opencode_config_paths_with_environment(target, &observed_environment(target)?)
}

/// Pure resolver; environment values must come from the selected runtime.
/// Ordering is for selecting the most specific editable file, not a merged-config claim.
pub fn opencode_config_paths_with_environment(
    target: &RuntimeTarget,
    values: &OpenCodeConfigEnvironment,
) -> Result<Vec<PathBuf>, StorageError> {
    let home = target.home_path.as_path();
    let mut paths = Vec::new();
    if let Some(directory) = &values.config_directory {
        let directory = runtime_path(target, directory)?;
        paths.extend([
            directory.join("opencode.jsonc"),
            directory.join("opencode.json"),
        ]);
    }
    if let Some(file) = &values.config_file {
        paths.push(runtime_path(target, file)?);
    }
    let global = match values
        .xdg_config_home
        .as_deref()
        .filter(|value| absolute_for_runtime(target, value))
    {
        Some(value) => runtime_path(target, value)?.join("opencode"),
        None => home.join(".config/opencode"),
    };
    paths.extend([
        global.join("opencode.jsonc"),
        global.join("opencode.json"),
        global.join("config.json"),
    ]);
    if target.platform == RuntimePlatform::Windows {
        let legacy = home.join("AppData/Roaming/opencode");
        paths.extend([legacy.join("opencode.jsonc"), legacy.join("opencode.json")]);
    }
    let mut seen = std::collections::BTreeSet::new();
    paths.retain(|path| seen.insert(config_path_key(target, path)));
    Ok(paths)
}

fn config_path_key(target: &RuntimeTarget, path: &Path) -> String {
    let mut text = path.to_string_lossy().into_owned();
    if target.platform == RuntimePlatform::Windows || text.starts_with(r"\\") {
        text = text.replace('\\', "/");
        if text.to_ascii_lowercase().starts_with("//?/unc/") {
            text = format!("//{}", &text[8..]);
        } else if text.starts_with("//?/") {
            text = text[4..].to_owned();
        }
        if text.to_ascii_lowercase().starts_with("//wsl.localhost/") {
            text = format!("//wsl$/{}", &text[16..]);
        }
    }
    if target.platform == RuntimePlatform::Windows {
        text.to_lowercase()
    } else {
        text
    }
}

/// An external scope is granted only to exact observed config paths and their
/// generated backup names, never to arbitrary files in the same directory.
pub(crate) fn observed_external_config_parent(
    target: &RuntimeTarget,
    path: &Path,
) -> Result<Option<PathBuf>, StorageError> {
    for candidate in opencode_config_paths(target)? {
        let is_backup = candidate.parent().map(|path| config_path_key(target, path))
            == path.parent().map(|path| config_path_key(target, path))
            && candidate
                .file_name()
                .and_then(|name| name.to_str())
                .zip(path.file_name().and_then(|name| name.to_str()))
                .is_some_and(|(name, file)| {
                    (if target.platform == RuntimePlatform::Windows {
                        file.to_lowercase()
                    } else {
                        file.to_owned()
                    })
                    .strip_prefix(&format!(
                        ".{}.vibehub.",
                        if target.platform == RuntimePlatform::Windows {
                            name.to_lowercase()
                        } else {
                            name.to_owned()
                        }
                    ))
                    .and_then(|suffix| suffix.strip_suffix(".bak"))
                    .is_some_and(|hash| {
                        hash.len() == 12 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
                });
        if config_path_key(target, path) == config_path_key(target, &candidate) || is_backup {
            return Ok(candidate.parent().map(Path::to_path_buf));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_verbatim_paths_match_and_linux_case_stays_distinct() {
        let mut target = RuntimeTarget::host(PathBuf::from("/home/test"));
        target.platform = RuntimePlatform::Windows;
        assert_eq!(
            config_path_key(&target, Path::new(r"C:\Config\OpenCode.json")),
            config_path_key(&target, Path::new(r"\\?\c:\config\opencode.json"))
        );
        assert_eq!(
            config_path_key(&target, Path::new(r"\\Server\Share\opencode.json")),
            config_path_key(&target, Path::new(r"\\?\UNC\server\share\opencode.json"))
        );
        let target = RuntimeTarget::wsl("Ubuntu", "/home/User");
        assert_ne!(
            config_path_key(&target, Path::new("/home/User/opencode.json")),
            config_path_key(&target, Path::new("/home/user/opencode.json"))
        );
        assert_eq!(
            config_path_key(&target, Path::new(r"\\wsl$\Ubuntu\home\User\opencode.json")),
            config_path_key(
                &target,
                Path::new(r"\\?\UNC\wsl.localhost\Ubuntu\home\User\opencode.json")
            )
        );
    }
}
