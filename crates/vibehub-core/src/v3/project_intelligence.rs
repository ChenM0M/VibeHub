use crate::process_util::silent_command;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub const DEFAULT_PAGE_SIZE: usize = 200;
pub const MAX_PAGE_SIZE: usize = 500;
const SNAPSHOT_SCHEMA: u32 = 1;
const ANALYZER_REGISTRY_VERSION: &str = "project-intelligence-1";
const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".turbo",
    ".vite",
    ".cache",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectModelSnapshot {
    pub schema_version: u32,
    pub model_version: String,
    pub generated_at: String,
    pub root: String,
    pub source_head: Option<String>,
    pub indexed_files: usize,
    pub nodes: Vec<ProjectNode>,
    #[serde(default)]
    pub analyzer_findings: Vec<AnalyzerFinding>,
    pub warnings: Vec<IndexWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzerFinding {
    pub analyzer: String,
    pub version: String,
    pub kind: String,
    pub path: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectNode {
    pub node_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub kind: NodeKind,
    pub relative_path: String,
    pub git_state: GitState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Root,
    Directory,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitState {
    Clean,
    Modified,
    Added,
    Deleted,
    Ignored,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexWarning {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidationUnit {
    TreePage,
    GitOverlay,
    ManifestWorkspace,
    ImportsSymbols,
    DeclaredDocumentation,
}

pub fn invalidation_units(path: &str) -> BTreeSet<InvalidationUnit> {
    let normalized = path.replace('\\', "/");
    let name = Path::new(&normalized)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut units = BTreeSet::from([InvalidationUnit::TreePage, InvalidationUnit::GitOverlay]);
    if matches!(
        name.as_str(),
        "cargo.toml" | "package.json" | "pyproject.toml"
    ) {
        units.insert(InvalidationUnit::ManifestWorkspace);
    }
    if normalized.starts_with("docs/adr/") || name.starts_with("readme") {
        units.insert(InvalidationUnit::DeclaredDocumentation);
    }
    if matches!(
        Path::new(&normalized)
            .extension()
            .and_then(|value| value.to_str()),
        Some("rs" | "ts" | "tsx" | "js" | "jsx" | "py")
    ) {
        units.insert(InvalidationUnit::ImportsSymbols);
    }
    units
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPage {
    pub snapshot: ProjectModelSnapshot,
    pub cursor: Option<String>,
    pub next_cursor: Option<String>,
    pub limit: usize,
    pub total_estimate: usize,
}

#[derive(Debug, Clone)]
pub struct ProjectIndexService {
    root: PathBuf,
    cached_index: Arc<RwLock<Option<ProjectModelSnapshot>>>,
}

impl ProjectIndexService {
    pub fn open(root: impl AsRef<Path>) -> std::io::Result<Self> {
        Ok(Self {
            root: root.as_ref().canonicalize()?,
            cached_index: Arc::new(RwLock::new(None)),
        })
    }

    pub fn first_page(&self) -> std::io::Result<ProjectPage> {
        self.page(".", None, DEFAULT_PAGE_SIZE)
    }

    pub fn search(&self, query: &str, limit: usize) -> std::io::Result<ProjectPage> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return self.first_page();
        }
        let limit = limit.clamp(1, MAX_PAGE_SIZE);
        if let Some(snapshot) = self
            .cached_index
            .read()
            .map_err(|_| std::io::Error::other("project index cache poisoned"))?
            .as_ref()
        {
            return Ok(search_snapshot(snapshot, &query, limit));
        }
        let head = git_output(&self.root, &["rev-parse", "HEAD"]);
        let status = git_status(&self.root);
        let model_version = model_version(&self.root, head.as_deref(), &status);
        let git_states = git_states_from_status(&status);
        let files = git_output(
            &self.root,
            &["ls-files", "--cached", "--others", "--exclude-standard"],
        )
        .unwrap_or_default();
        let matches: Vec<String> = files
            .lines()
            .map(|path| path.replace('\\', "/"))
            .filter(|path| path.to_ascii_lowercase().contains(&query))
            .take(limit)
            .collect();
        let root_name = self
            .root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Project")
            .to_owned();
        let mut nodes = vec![ProjectNode {
            node_id: node_id("."),
            parent_id: None,
            name: root_name,
            kind: NodeKind::Root,
            relative_path: ".".into(),
            git_state: directory_git_state(".", &git_states),
        }];
        nodes.extend(matches.iter().map(|path| {
            ProjectNode {
                node_id: node_id(path),
                parent_id: Some(node_id(".")),
                name: Path::new(path)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(path)
                    .to_owned(),
                kind: NodeKind::File,
                relative_path: path.clone(),
                git_state: git_states.get(path).copied().unwrap_or(GitState::Clean),
            }
        }));
        Ok(ProjectPage {
            snapshot: ProjectModelSnapshot {
                schema_version: SNAPSHOT_SCHEMA,
                model_version,
                generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                root: self.root.to_string_lossy().into_owned(),
                source_head: head,
                indexed_files: git_tracked_count(&self.root),
                analyzer_findings: analyze_declared_project(&self.root),
                nodes,
                warnings: Vec::new(),
            },
            cursor: None,
            next_cursor: None,
            limit,
            total_estimate: matches.len(),
        })
    }

    pub fn full_index(&self) -> std::io::Result<ProjectModelSnapshot> {
        let head = git_output(&self.root, &["rev-parse", "HEAD"]);
        let status = git_status(&self.root);
        let model_version = model_version(&self.root, head.as_deref(), &status);
        let git_states = git_states_from_status(&status);
        let listing = git_output(
            &self.root,
            &["ls-files", "--cached", "--others", "--exclude-standard"],
        )
        .unwrap_or_default();
        let files: Vec<String> = listing
            .lines()
            .map(|path| path.replace('\\', "/"))
            .filter(|path| !path.is_empty())
            .collect();
        let mut directories = BTreeSet::new();
        for file in &files {
            let mut parent = Path::new(file).parent();
            while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
                directories.insert(relative_display(path));
                parent = path.parent();
            }
        }
        let mut nodes = vec![ProjectNode {
            node_id: node_id("."),
            parent_id: None,
            name: self
                .root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Project")
                .into(),
            kind: NodeKind::Root,
            relative_path: ".".into(),
            git_state: directory_git_state(".", &git_states),
        }];
        nodes.extend(directories.iter().map(|path| {
            ProjectNode {
                node_id: node_id(path),
                parent_id: Some(node_id(parent_path(path))),
                name: Path::new(path)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(path)
                    .into(),
                kind: NodeKind::Directory,
                relative_path: path.clone(),
                git_state: directory_git_state(path, &git_states),
            }
        }));
        nodes.extend(files.iter().map(|path| {
            ProjectNode {
                node_id: node_id(path),
                parent_id: Some(node_id(parent_path(path))),
                name: Path::new(path)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(path)
                    .into(),
                kind: NodeKind::File,
                relative_path: path.clone(),
                git_state: git_states.get(path).copied().unwrap_or(GitState::Clean),
            }
        }));
        let snapshot = ProjectModelSnapshot {
            schema_version: SNAPSHOT_SCHEMA,
            model_version,
            generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            root: self.root.to_string_lossy().into_owned(),
            source_head: head,
            indexed_files: files.len(),
            nodes,
            analyzer_findings: analyze_project(&self.root),
            warnings: Vec::new(),
        };
        *self
            .cached_index
            .write()
            .map_err(|_| std::io::Error::other("project index cache poisoned"))? =
            Some(snapshot.clone());
        Ok(snapshot)
    }

    pub fn page(
        &self,
        relative_dir: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> std::io::Result<ProjectPage> {
        let limit = limit.clamp(1, MAX_PAGE_SIZE);
        let relative_dir = safe_relative_dir(relative_dir)?;
        let head = git_output(&self.root, &["rev-parse", "HEAD"]);
        let status = git_status(&self.root);
        let model_version = model_version(&self.root, head.as_deref(), &status);
        let offset = decode_cursor(cursor, &model_version, &relative_dir)?;
        let git_states = git_states_from_status(&status);
        let mut warnings = Vec::new();
        let mut entries = Vec::new();
        let directory = self.root.join(&relative_dir).canonicalize()?;
        if !directory.starts_with(&self.root) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "directory escapes project root through a symbolic link",
            ));
        }
        for item in fs::read_dir(&directory)? {
            match item {
                Ok(item) => {
                    let path = item.path();
                    let name = item.file_name().to_string_lossy().into_owned();
                    if IGNORED_DIRS.contains(&name.as_str()) {
                        continue;
                    }
                    match fs::symlink_metadata(&path) {
                        Ok(metadata) if metadata.file_type().is_symlink() => {
                            warnings.push(IndexWarning {
                                code: "PI_SYMLINK_SKIPPED".into(),
                                path: relative_display(&relative_dir.join(&name)),
                                message: "Symbolic link was not followed".into(),
                            });
                        }
                        Ok(metadata) => entries.push((path, name, metadata.is_dir())),
                        Err(error) => warnings.push(IndexWarning {
                            code: "PI_METADATA_UNAVAILABLE".into(),
                            path: relative_display(&relative_dir.join(&name)),
                            message: error.to_string(),
                        }),
                    }
                }
                Err(error) => warnings.push(IndexWarning {
                    code: "PI_ENTRY_UNAVAILABLE".into(),
                    path: relative_display(&relative_dir),
                    message: error.to_string(),
                }),
            }
        }
        entries
            .sort_by_key(|(_, name, is_dir)| (!*is_dir, name.to_ascii_lowercase(), name.clone()));
        let total = entries.len();
        let root_name = self
            .root
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("Project")
            .to_string();
        let root_relative = relative_display(&relative_dir);
        let parent_id = node_id(&root_relative);
        let mut nodes = vec![ProjectNode {
            node_id: parent_id.clone(),
            parent_id: None,
            name: if root_relative == "." {
                root_name
            } else {
                relative_dir
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("directory")
                    .into()
            },
            kind: if root_relative == "." {
                NodeKind::Root
            } else {
                NodeKind::Directory
            },
            relative_path: root_relative.clone(),
            git_state: directory_git_state(&root_relative, &git_states),
        }];
        for (_, name, is_dir) in entries.iter().skip(offset).take(limit) {
            let relative = relative_display(&relative_dir.join(name));
            nodes.push(ProjectNode {
                node_id: node_id(&relative),
                parent_id: Some(parent_id.clone()),
                name: name.clone(),
                kind: if *is_dir {
                    NodeKind::Directory
                } else {
                    NodeKind::File
                },
                relative_path: relative.clone(),
                git_state: git_states
                    .get(&relative)
                    .copied()
                    .unwrap_or_else(|| directory_git_state(&relative, &git_states)),
            });
        }
        let next_offset = offset + nodes.len().saturating_sub(1);
        let next_cursor = (next_offset < total)
            .then(|| encode_cursor(&model_version, &relative_dir, next_offset));
        Ok(ProjectPage {
            snapshot: ProjectModelSnapshot {
                schema_version: SNAPSHOT_SCHEMA,
                model_version,
                generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                root: self.root.to_string_lossy().into_owned(),
                source_head: head,
                indexed_files: git_tracked_count(&self.root),
                analyzer_findings: analyze_declared_project(&self.root),
                nodes,
                warnings,
            },
            cursor: cursor.map(str::to_owned),
            next_cursor,
            limit,
            total_estimate: total,
        })
    }

    pub fn publish_snapshot(
        &self,
        snapshot: &ProjectModelSnapshot,
        destination: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let destination = destination.as_ref();
        let parent = destination.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "snapshot destination has no parent",
            )
        })?;
        fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(
            ".{}.{}.{}.tmp",
            destination
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("snapshot"),
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let bytes = serde_json::to_vec(snapshot).map_err(std::io::Error::other)?;
        let mut file = fs::File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        if let Err(error) = replace_file(&temporary, destination) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        sync_directory(parent)?;
        Ok(())
    }

    pub fn load_snapshot(&self, source: impl AsRef<Path>) -> std::io::Result<ProjectModelSnapshot> {
        let snapshot: ProjectModelSnapshot =
            serde_json::from_slice(&fs::read(source)?).map_err(std::io::Error::other)?;
        if snapshot.schema_version != SNAPSHOT_SCHEMA
            || snapshot.root != self.root.to_string_lossy()
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "snapshot identity or schema mismatch",
            ));
        }
        Ok(snapshot)
    }
}

fn search_snapshot(snapshot: &ProjectModelSnapshot, query: &str, limit: usize) -> ProjectPage {
    let root = snapshot.nodes.first().cloned().unwrap_or(ProjectNode {
        node_id: node_id("."),
        parent_id: None,
        name: "Project".into(),
        kind: NodeKind::Root,
        relative_path: ".".into(),
        git_state: GitState::Clean,
    });
    let matches: Vec<ProjectNode> = snapshot
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::File)
        .filter(|node| node.relative_path.to_ascii_lowercase().contains(query))
        .take(limit)
        .map(|node| {
            let mut node = node.clone();
            node.parent_id = Some(root.node_id.clone());
            node
        })
        .collect();
    let total_estimate = matches.len();
    let mut nodes = Vec::with_capacity(matches.len() + 1);
    nodes.push(root);
    nodes.extend(matches);
    ProjectPage {
        snapshot: ProjectModelSnapshot {
            nodes,
            analyzer_findings: snapshot.analyzer_findings.clone(),
            warnings: snapshot.warnings.clone(),
            ..snapshot.clone()
        },
        cursor: None,
        next_cursor: None,
        limit,
        total_estimate,
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> std::io::Result<()> {
    fs::File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn safe_relative_dir(value: &str) -> std::io::Result<PathBuf> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir
                    | std::path::Component::Prefix(_)
                    | std::path::Component::RootDir
            )
        })
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "directory escapes project root",
        ));
    }
    Ok(if value.is_empty() || value == "." {
        PathBuf::from(".")
    } else {
        path.to_path_buf()
    })
}

fn model_version(root: &Path, head: Option<&str>, status: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(root.to_string_lossy().as_bytes());
    digest.update(head.unwrap_or("worktree").as_bytes());
    digest.update(status.as_bytes());
    for line in status.lines().filter(|line| line.len() >= 4) {
        if let Some(path) = line[3..].split(" -> ").last() {
            match fs::metadata(root.join(path)) {
                Ok(metadata) => {
                    digest.update(metadata.len().to_le_bytes());
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(since_epoch) = modified.duration_since(std::time::UNIX_EPOCH) {
                            digest.update(since_epoch.as_nanos().to_le_bytes());
                        }
                    }
                }
                Err(error) => digest.update(error.to_string().as_bytes()),
            }
        }
    }
    digest.update(ANALYZER_REGISTRY_VERSION.as_bytes());
    digest.update(IGNORED_DIRS.join("\0").as_bytes());
    format!("pi-{:x}", digest.finalize())
}

fn analyze_project(root: &Path) -> Vec<AnalyzerFinding> {
    let mut findings = analyze_declared_project(root);
    let source_files = git_output(
        root,
        &["ls-files", "--cached", "--others", "--exclude-standard"],
    )
    .unwrap_or_default();
    let mut capabilities = BTreeSet::new();
    for path in source_files.lines() {
        let extension = Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let analyzer = match extension {
            "rs" => "rust_imports",
            "ts" | "tsx" | "js" | "jsx" => "javascript_imports",
            "py" => "python_imports",
            _ => continue,
        };
        capabilities.insert(analyzer);
        let full_path = root.join(path);
        let Ok(metadata) = fs::metadata(&full_path) else {
            continue;
        };
        if metadata.len() > 1_048_576 {
            continue;
        }
        let Ok(content) = fs::read_to_string(&full_path) else {
            continue;
        };
        for line in content
            .lines()
            .map(str::trim)
            .filter(|line| static_import_line(analyzer, line))
            .take(128)
        {
            findings.push(AnalyzerFinding {
                analyzer: analyzer.into(),
                version: "1".into(),
                kind: "static_import".into(),
                path: path.replace('\\', "/"),
                label: line.chars().take(240).collect(),
            });
        }
    }
    findings.extend(capabilities.into_iter().map(|analyzer| AnalyzerFinding {
        analyzer: analyzer.into(),
        version: "1".into(),
        kind: "capability".into(),
        path: ".".into(),
        label: "Static import extraction available".into(),
    }));
    findings
}

fn analyze_declared_project(root: &Path) -> Vec<AnalyzerFinding> {
    const CANDIDATES: &[(&str, &str, &str)] = &[
        ("Cargo.toml", "cargo", "manifest"),
        ("package.json", "npm", "manifest"),
        ("pyproject.toml", "python", "manifest"),
        ("README.md", "documentation", "readme"),
        ("README", "documentation", "readme"),
    ];
    let mut findings = Vec::new();
    for (path, analyzer, kind) in CANDIDATES {
        if root.join(path).is_file() {
            findings.push(AnalyzerFinding {
                analyzer: (*analyzer).into(),
                version: "1".into(),
                kind: (*kind).into(),
                path: (*path).into(),
                label: root
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("project")
                    .into(),
            });
        }
    }
    let adr = root.join("docs/adr");
    if adr.is_dir() {
        findings.push(AnalyzerFinding {
            analyzer: "documentation".into(),
            version: "1".into(),
            kind: "adr_directory".into(),
            path: "docs/adr".into(),
            label: "Architecture decisions".into(),
        });
    }
    findings
}

fn static_import_line(analyzer: &str, line: &str) -> bool {
    match analyzer {
        "rust_imports" => {
            line.starts_with("use ")
                || line.starts_with("pub use ")
                || line.starts_with("mod ")
                || line.starts_with("pub mod ")
        }
        "javascript_imports" => {
            (line.starts_with("import ") && !line.starts_with("import("))
                || line.starts_with("export ") && line.contains(" from ")
        }
        "python_imports" => line.starts_with("import ") || line.starts_with("from "),
        _ => false,
    }
}

fn git_status(root: &Path) -> String {
    git_output(root, &["status", "--porcelain=v1", "--untracked-files=all"]).unwrap_or_default()
}

fn git_states_from_status(status: &str) -> BTreeMap<String, GitState> {
    status
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let status = &line[..2];
            let path = line[3..].split(" -> ").last()?.replace('\\', "/");
            let state = if status == "??" {
                GitState::Added
            } else if status.contains('D') {
                GitState::Deleted
            } else if status.contains('A') {
                GitState::Added
            } else {
                GitState::Modified
            };
            Some((path, state))
        })
        .collect()
}

fn directory_git_state(path: &str, states: &BTreeMap<String, GitState>) -> GitState {
    if path == "." {
        return if states.is_empty() {
            GitState::Clean
        } else {
            GitState::Modified
        };
    }
    let prefix = format!("{path}/");
    if states
        .keys()
        .any(|candidate| candidate == path || candidate.starts_with(&prefix))
    {
        GitState::Modified
    } else {
        GitState::Clean
    }
}

fn git_tracked_count(root: &Path) -> usize {
    git_output(root, &["ls-files"])
        .map(|v| v.lines().count())
        .unwrap_or(0)
}
fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = silent_command("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned()
    })
}
fn relative_display(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if value.is_empty() {
        ".".into()
    } else {
        value.trim_start_matches("./").to_owned()
    }
    .pipe(|v| if v.is_empty() { ".".into() } else { v })
}
fn node_id(path: &str) -> String {
    if path == "." {
        "path.project-root".into()
    } else {
        format!("path.{}", path.replace(['/', '\\'], "."))
    }
}
fn parent_path(path: &str) -> &str {
    Path::new(path)
        .parent()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(".")
}
fn encode_cursor(model: &str, directory: &Path, offset: usize) -> String {
    format!(
        "{model}:{}:{offset}",
        relative_display(directory).replace(':', "%3A")
    )
}
fn decode_cursor(cursor: Option<&str>, model: &str, directory: &Path) -> std::io::Result<usize> {
    let Some(cursor) = cursor else { return Ok(0) };
    let prefix = format!(
        "{model}:{}:",
        relative_display(directory).replace(':', "%3A")
    );
    cursor
        .strip_prefix(&prefix)
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cursor does not belong to this model or directory",
            )
        })
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-pi-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn pages_stably_and_ignores_heavy_directories() {
        let root = project();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("a.txt"), "a").unwrap();
        fs::write(root.join("b.txt"), "b").unwrap();
        let service = ProjectIndexService::open(&root).unwrap();
        let first = service.page(".", None, 2).unwrap();
        assert_eq!(first.snapshot.nodes.len(), 3);
        assert!(first
            .snapshot
            .nodes
            .iter()
            .all(|node| node.name != "node_modules"));
        assert!(first.next_cursor.is_some());
        let second = service.page(".", first.next_cursor.as_deref(), 2).unwrap();
        assert_ne!(
            first.snapshot.nodes[1].relative_path,
            second.snapshot.nodes[1].relative_path
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn rejects_cursor_from_another_model() {
        let root = project();
        let service = ProjectIndexService::open(&root).unwrap();
        assert!(service.page(".", Some("pi-old:.:1"), 10).is_err());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn model_version_changes_when_dirty_file_contents_change() {
        let root = project();
        let run_git = |args: &[&str]| {
            let status = silent_command("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {args:?}");
        };
        run_git(&["init", "--quiet"]);
        run_git(&["config", "user.email", "test@example.com"]);
        run_git(&["config", "user.name", "VibeHub Test"]);
        fs::write(root.join("tracked.txt"), "base").unwrap();
        run_git(&["add", "tracked.txt"]);
        run_git(&["commit", "--quiet", "-m", "base"]);

        fs::write(root.join("tracked.txt"), "first").unwrap();
        let service = ProjectIndexService::open(&root).unwrap();
        let first = service.first_page().unwrap().snapshot.model_version;
        fs::write(root.join("tracked.txt"), "second").unwrap();
        let second = service.first_page().unwrap().snapshot.model_version;

        assert_ne!(first, second);
        fs::remove_dir_all(root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_directory_that_escapes_through_symlink() {
        use std::os::unix::fs::symlink;

        let root = project();
        let outside = project();
        fs::write(outside.join("secret.txt"), "secret").unwrap();
        symlink(&outside, root.join("outside")).unwrap();
        let service = ProjectIndexService::open(&root).unwrap();

        let error = service.page("outside", None, 10).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);

        fs::remove_dir_all(root).ok();
        fs::remove_dir_all(outside).ok();
    }

    #[test]
    fn publishes_a_readable_snapshot_atomically() {
        let root = project();
        fs::write(root.join("file.txt"), "content").unwrap();
        let service = ProjectIndexService::open(&root).unwrap();
        let snapshot = service.first_page().unwrap().snapshot;
        let destination = root.join("cache/snapshot.json");
        service.publish_snapshot(&snapshot, &destination).unwrap();
        let loaded: ProjectModelSnapshot =
            serde_json::from_slice(&fs::read(&destination).unwrap()).unwrap();
        assert_eq!(loaded.model_version, snapshot.model_version);
        assert_eq!(service.load_snapshot(&destination).unwrap(), snapshot);

        fs::write(root.join("second.txt"), "new content").unwrap();
        let replacement = service.full_index().unwrap();
        service
            .publish_snapshot(&replacement, &destination)
            .unwrap();
        assert_eq!(service.load_snapshot(&destination).unwrap(), replacement);
        assert!(!fs::read_dir(destination.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().ends_with(".tmp")));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn rejects_corrupt_or_foreign_snapshot() {
        let root = project();
        let service = ProjectIndexService::open(&root).unwrap();
        let source = root.join("bad.json");
        fs::write(&source, b"not-json").unwrap();
        assert!(service.load_snapshot(source).is_err());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn discovers_supported_manifests_and_documentation() {
        let root = project();
        fs::write(root.join("Cargo.toml"), "[package]\nname='demo'").unwrap();
        fs::write(root.join("README.md"), "# Demo").unwrap();
        let snapshot = ProjectIndexService::open(&root)
            .unwrap()
            .first_page()
            .unwrap()
            .snapshot;
        assert!(snapshot
            .analyzer_findings
            .iter()
            .any(|item| item.analyzer == "cargo"));
        assert!(snapshot
            .analyzer_findings
            .iter()
            .any(|item| item.kind == "readme"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn full_index_builds_directory_ancestry_without_duplicates() {
        let root = project();
        fs::create_dir_all(root.join("src/nested")).unwrap();
        fs::write(root.join("src/nested/mod.rs"), "").unwrap();
        silent_command("git")
            .arg("-C")
            .arg(&root)
            .arg("init")
            .output()
            .unwrap();
        let snapshot = ProjectIndexService::open(&root)
            .unwrap()
            .full_index()
            .unwrap();
        assert!(snapshot
            .nodes
            .iter()
            .any(|node| node.relative_path == "src"));
        assert!(snapshot
            .nodes
            .iter()
            .any(|node| node.relative_path == "src/nested/mod.rs"));
        let ids: BTreeSet<_> = snapshot.nodes.iter().map(|node| &node.node_id).collect();
        assert_eq!(ids.len(), snapshot.nodes.len());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn searches_the_cached_full_index() {
        let root = project();
        fs::create_dir_all(root.join("src/nested")).unwrap();
        fs::write(root.join("src/nested/model.rs"), "pub struct Model;").unwrap();
        fs::write(root.join("README.md"), "# Demo").unwrap();
        silent_command("git")
            .arg("-C")
            .arg(&root)
            .arg("init")
            .output()
            .unwrap();
        let service = ProjectIndexService::open(&root).unwrap();
        let full = service.full_index().unwrap();
        let result = service.search("model.rs", 10).unwrap();

        assert_eq!(result.snapshot.model_version, full.model_version);
        assert_eq!(result.snapshot.nodes.len(), 2);
        assert_eq!(
            result.snapshot.nodes[1].relative_path,
            "src/nested/model.rs"
        );
        assert_eq!(
            result.snapshot.nodes[1].parent_id,
            Some(result.snapshot.nodes[0].node_id.clone())
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn invalidation_is_scoped_by_evidence_kind() {
        assert_eq!(
            invalidation_units("assets/logo.png"),
            BTreeSet::from([InvalidationUnit::TreePage, InvalidationUnit::GitOverlay])
        );
        assert!(invalidation_units("Cargo.toml").contains(&InvalidationUnit::ManifestWorkspace));
        assert!(invalidation_units("src/lib.rs").contains(&InvalidationUnit::ImportsSymbols));
        assert!(invalidation_units("docs/adr/001.md")
            .contains(&InvalidationUnit::DeclaredDocumentation));
    }

    #[test]
    fn extracts_only_supported_static_import_forms() {
        assert!(static_import_line("rust_imports", "use crate::model;"));
        assert!(static_import_line(
            "javascript_imports",
            "import { x } from './x';"
        ));
        assert!(!static_import_line(
            "javascript_imports",
            "import('./lazy')"
        ));
        assert!(static_import_line(
            "python_imports",
            "from app import model"
        ));
    }
}
