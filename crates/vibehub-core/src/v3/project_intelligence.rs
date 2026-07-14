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
pub const PROJECT_MODEL_INDEX_PATH: &str = ".vibehub/indexes/project-model.json";
const SNAPSHOT_SCHEMA: u32 = 1;
const ANALYZER_REGISTRY_VERSION: &str = "project-intelligence-2";
const ANALYZER_FINDING_VERSION: &str = "2";
const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".vibehub",
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AnalyzerFinding {
    pub analyzer: String,
    pub version: String,
    /// Stable finding category. Relationship findings use `target` and, when
    /// the target is local, `target_path` to identify the other endpoint.
    pub kind: String,
    /// Project-relative evidence/source path for this observation.
    pub path: String,
    pub label: String,
    /// Canonical package, crate, module, or import identifier when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Resolved project-relative target path. Missing means external or not
    /// safely resolvable; old snapshots deserialize it as `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
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
    let lower = normalized.to_ascii_lowercase();
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
    let document_extension = Path::new(&lower)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| matches!(value, "md" | "mdx" | "rst" | "txt" | "adoc"));
    if is_under_path(&lower, "docs/adr")
        || is_under_path(&lower, "docs/rfc")
        || is_under_path(&lower, "docs/v3/rfc-backlog")
        || name.starts_with("readme")
        || (document_extension && (lower.contains("architecture") || lower.contains("redesign")))
    {
        units.insert(InvalidationUnit::DeclaredDocumentation);
    }
    if Path::new(&lower)
        .extension()
        .and_then(|value| value.to_str())
        .and_then(source_capability)
        .is_some()
    {
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

    pub fn snapshot_path(&self) -> PathBuf {
        self.root.join(PROJECT_MODEL_INDEX_PATH)
    }

    /// Load the current JSON project model or rebuild and atomically publish it
    /// when the persisted fingerprint no longer matches the working tree.
    pub fn current_snapshot(&self) -> std::io::Result<ProjectModelSnapshot> {
        let head = git_output(&self.root, &["rev-parse", "HEAD"]);
        let status = git_status(&self.root);
        let expected_model_version = model_version(&self.root, head.as_deref(), &status);
        let destination = self.snapshot_path();

        if let Ok(snapshot) = self.load_snapshot(&destination) {
            if snapshot.model_version == expected_model_version {
                *self
                    .cached_index
                    .write()
                    .map_err(|_| std::io::Error::other("project index cache poisoned"))? =
                    Some(snapshot.clone());
                return Ok(snapshot);
            }
        }

        let snapshot = self.full_index()?;
        self.publish_snapshot(&snapshot, &destination)?;
        let persisted = self.load_snapshot(&destination)?;
        *self
            .cached_index
            .write()
            .map_err(|_| std::io::Error::other("project index cache poisoned"))? =
            Some(persisted.clone());
        Ok(persisted)
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
        let matches: Vec<String> = project_files(&self.root)
            .into_iter()
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
                analyzer_findings: analyze_project(&self.root),
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
        let files = project_files(&self.root);
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
                analyzer_findings: analyze_project(&self.root),
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
    let files = project_files(root);
    let mut findings = analyze_declared_project(root, &files);
    let local_targets = local_target_paths(&findings);
    let mut capabilities = BTreeMap::<String, bool>::new();

    for path in &files {
        let extension = Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let Some((analyzer, supported)) = source_capability(&extension) else {
            continue;
        };
        capabilities
            .entry(analyzer.to_owned())
            .and_modify(|value| *value |= supported)
            .or_insert(supported);
        if !supported {
            continue;
        }

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
            .filter(|line| !line.is_empty())
        {
            let Some((target, target_path)) =
                resolve_static_import(root, path, analyzer, line, &local_targets)
            else {
                continue;
            };
            findings.push(analyzer_finding(
                analyzer,
                "static_import",
                path,
                line.chars().take(240).collect::<String>(),
                Some(target),
                target_path,
            ));
        }
    }

    findings.extend(capabilities.into_iter().map(|(analyzer, supported)| {
        analyzer_finding(
            &analyzer,
            "capability",
            ".",
            if supported {
                "Static import extraction available"
            } else {
                "Static import extraction unsupported"
            },
            Some(
                if supported {
                    "supported"
                } else {
                    "unsupported"
                }
                .to_owned(),
            ),
            None,
        )
    }));
    findings.sort();
    findings.dedup();
    findings
}

fn analyzer_finding(
    analyzer: &str,
    kind: &str,
    path: &str,
    label: impl Into<String>,
    target: Option<String>,
    target_path: Option<String>,
) -> AnalyzerFinding {
    AnalyzerFinding {
        analyzer: analyzer.to_owned(),
        version: ANALYZER_FINDING_VERSION.to_owned(),
        kind: kind.to_owned(),
        path: path.replace('\\', "/"),
        label: label.into(),
        target,
        target_path,
    }
}

fn analyze_declared_project(root: &Path, files: &[String]) -> Vec<AnalyzerFinding> {
    let mut findings = Vec::new();
    analyze_cargo_manifests(root, files, &mut findings);
    analyze_npm_manifests(root, files, &mut findings);
    analyze_python_manifests(root, files, &mut findings);
    analyze_architecture_documents(root, files, &mut findings);
    findings
}

fn analyze_cargo_manifests(root: &Path, files: &[String], findings: &mut Vec<AnalyzerFinding>) {
    let mut manifests = BTreeMap::<String, toml::Value>::new();
    for path in files.iter().filter(|path| {
        Path::new(path).file_name().and_then(|value| value.to_str()) == Some("Cargo.toml")
    }) {
        match fs::read_to_string(root.join(path))
            .map_err(|error| error.to_string())
            .and_then(|content| {
                toml::from_str::<toml::Value>(&content).map_err(|error| error.to_string())
            }) {
            Ok(value) => {
                manifests.insert(path.clone(), value);
            }
            Err(error) => findings.push(analyzer_finding(
                "cargo",
                "analyzer_error",
                path,
                format!("Manifest parse failed: {}", truncate(&error, 240)),
                None,
                None,
            )),
        }
    }

    let mut packages_by_dir = BTreeMap::<String, String>::new();
    let mut packages_by_name = BTreeMap::<String, String>::new();
    for (path, value) in &manifests {
        let directory = manifest_directory(path);
        if let Some(name) = cargo_package_name(value) {
            packages_by_dir.insert(directory.clone(), name.clone());
            packages_by_name
                .entry(name.clone())
                .or_insert(directory.clone());
            findings.push(analyzer_finding(
                "cargo",
                "package_manifest",
                path,
                &name,
                Some(name.clone()),
                Some(directory.clone()),
            ));
        }
        if value
            .get("workspace")
            .and_then(toml::Value::as_table)
            .is_some()
        {
            let label =
                cargo_package_name(value).unwrap_or_else(|| project_label(root, &directory));
            findings.push(analyzer_finding(
                "cargo",
                "workspace_manifest",
                path,
                label,
                None,
                Some(directory),
            ));
        }
    }

    for (path, value) in &manifests {
        let Some(workspace) = value.get("workspace").and_then(toml::Value::as_table) else {
            continue;
        };
        let workspace_dir = manifest_directory(path);
        let members = string_array(workspace.get("members"));
        let excludes = string_array(workspace.get("exclude"));
        for (directory, name) in &packages_by_dir {
            if directory == &workspace_dir {
                continue;
            }
            let relative = relative_to_directory(&workspace_dir, directory);
            if matches_any_path_pattern(&members, &relative)
                && !matches_any_path_pattern(&excludes, &relative)
            {
                findings.push(analyzer_finding(
                    "cargo",
                    "workspace_member",
                    path,
                    name,
                    Some(name.clone()),
                    Some(directory.clone()),
                ));
            }
        }
    }

    for (path, value) in &manifests {
        let directory = manifest_directory(path);
        for (name, dependency) in cargo_dependencies(value) {
            let (dependency_directory, effective_dependency) = if dependency
                .as_table()
                .and_then(|table| table.get("workspace"))
                .and_then(toml::Value::as_bool)
                == Some(true)
            {
                nearest_workspace_dependency(&manifests, &directory, name)
                    .unwrap_or_else(|| (directory.clone(), dependency))
            } else {
                (directory.clone(), dependency)
            };
            let declared_target = effective_dependency
                .as_table()
                .and_then(|table| table.get("package"))
                .and_then(toml::Value::as_str)
                .unwrap_or(name)
                .to_owned();
            let declared_path = effective_dependency
                .as_table()
                .and_then(|table| table.get("path"))
                .and_then(toml::Value::as_str);
            let mut target_path = declared_path
                .and_then(|path| resolve_relative_path(&dependency_directory, path))
                .and_then(|path| cargo_package_directory(&path, &packages_by_dir));
            if target_path.is_none() {
                target_path = packages_by_name
                    .get(&declared_target)
                    .or_else(|| packages_by_name.get(name))
                    .cloned();
            }
            let target = target_path
                .as_ref()
                .and_then(|path| packages_by_dir.get(path))
                .cloned()
                .unwrap_or(declared_target);
            findings.push(analyzer_finding(
                "cargo",
                "dependency",
                path,
                name,
                Some(target),
                target_path,
            ));
        }
    }
}

fn analyze_npm_manifests(root: &Path, files: &[String], findings: &mut Vec<AnalyzerFinding>) {
    let mut manifests = BTreeMap::<String, serde_json::Value>::new();
    for path in files.iter().filter(|path| {
        Path::new(path).file_name().and_then(|value| value.to_str()) == Some("package.json")
    }) {
        match fs::read_to_string(root.join(path))
            .map_err(|error| error.to_string())
            .and_then(|content| serde_json::from_str(&content).map_err(|error| error.to_string()))
        {
            Ok(value) => {
                manifests.insert(path.clone(), value);
            }
            Err(error) => findings.push(analyzer_finding(
                "npm",
                "analyzer_error",
                path,
                format!("Manifest parse failed: {}", truncate(&error, 240)),
                None,
                None,
            )),
        }
    }

    let mut packages_by_dir = BTreeMap::<String, String>::new();
    let mut packages_by_name = BTreeMap::<String, String>::new();
    for (path, value) in &manifests {
        let directory = manifest_directory(path);
        let name = value
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| project_label(root, &directory));
        packages_by_dir.insert(directory.clone(), name.clone());
        packages_by_name
            .entry(name.clone())
            .or_insert(directory.clone());
        findings.push(analyzer_finding(
            "npm",
            "package_manifest",
            path,
            &name,
            Some(name.clone()),
            Some(directory.clone()),
        ));
        if value.get("workspaces").is_some() {
            findings.push(analyzer_finding(
                "npm",
                "workspace_manifest",
                path,
                value
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_else(|| packages_by_dir[&directory].as_str()),
                None,
                Some(directory),
            ));
        }
    }

    for (path, value) in &manifests {
        let workspace_dir = manifest_directory(path);
        let members = npm_workspace_patterns(value);
        if members.is_empty() {
            continue;
        }
        for (directory, name) in &packages_by_dir {
            if directory == &workspace_dir {
                continue;
            }
            let relative = relative_to_directory(&workspace_dir, directory);
            if matches_any_path_pattern(&members, &relative) {
                findings.push(analyzer_finding(
                    "npm",
                    "workspace_member",
                    path,
                    name,
                    Some(name.clone()),
                    Some(directory.clone()),
                ));
            }
        }
    }

    for (path, value) in &manifests {
        let directory = manifest_directory(path);
        for section in [
            "dependencies",
            "devDependencies",
            "peerDependencies",
            "optionalDependencies",
        ] {
            let Some(dependencies) = value.get(section).and_then(serde_json::Value::as_object)
            else {
                continue;
            };
            for (name, requirement) in dependencies {
                let requirement = requirement.as_str().unwrap_or("");
                let mut target = npm_alias_target(requirement).unwrap_or_else(|| name.clone());
                let mut target_path = npm_local_path(requirement)
                    .and_then(|path| resolve_relative_path(&directory, path))
                    .and_then(|path| npm_package_directory(&path, &packages_by_dir));
                if target_path.is_none() {
                    target_path = packages_by_name
                        .get(&target)
                        .or_else(|| packages_by_name.get(name))
                        .cloned();
                }
                if let Some(package_name) = target_path
                    .as_ref()
                    .and_then(|path| packages_by_dir.get(path))
                {
                    target = package_name.clone();
                }
                findings.push(analyzer_finding(
                    "npm",
                    "dependency",
                    path,
                    name,
                    Some(target),
                    target_path,
                ));
            }
        }
    }
}

fn analyze_python_manifests(root: &Path, files: &[String], findings: &mut Vec<AnalyzerFinding>) {
    for path in files.iter().filter(|path| {
        Path::new(path).file_name().and_then(|value| value.to_str()) == Some("pyproject.toml")
    }) {
        let value = match fs::read_to_string(root.join(path))
            .map_err(|error| error.to_string())
            .and_then(|content| {
                toml::from_str::<toml::Value>(&content).map_err(|error| error.to_string())
            }) {
            Ok(value) => value,
            Err(error) => {
                findings.push(analyzer_finding(
                    "python",
                    "analyzer_error",
                    path,
                    format!("Manifest parse failed: {}", truncate(&error, 240)),
                    None,
                    None,
                ));
                continue;
            }
        };
        let directory = manifest_directory(path);
        let name = value
            .get("project")
            .and_then(|value| value.get("name"))
            .or_else(|| {
                value
                    .get("tool")
                    .and_then(|value| value.get("poetry"))
                    .and_then(|value| value.get("name"))
            })
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| project_label(root, &directory));
        findings.push(analyzer_finding(
            "python",
            "package_manifest",
            path,
            &name,
            Some(name.clone()),
            Some(directory),
        ));
    }
}

fn analyze_architecture_documents(
    root: &Path,
    files: &[String],
    findings: &mut Vec<AnalyzerFinding>,
) {
    for path in files {
        let lower = path.to_ascii_lowercase();
        let is_readme = !lower.contains('/')
            && matches!(
                lower.as_str(),
                "readme" | "readme.md" | "readme.rst" | "readme.txt" | "readme.adoc"
            );
        let extension = Path::new(&lower)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let document = matches!(extension, "md" | "mdx" | "rst" | "txt" | "adoc");
        let declared_architecture = document
            && (is_under_path(&lower, "docs/adr")
                || is_under_path(&lower, "docs/rfc")
                || is_under_path(&lower, "docs/v3/rfc-backlog")
                || lower.contains("architecture")
                || lower.contains("redesign"));
        let kind = if is_readme {
            "readme"
        } else if declared_architecture {
            "architecture_document"
        } else {
            continue;
        };
        findings.push(analyzer_finding(
            "documentation",
            kind,
            path,
            document_label(root, path),
            None,
            Some(path.clone()),
        ));
    }
}

fn cargo_package_name(value: &toml::Value) -> Option<String> {
    value
        .get("package")?
        .get("name")?
        .as_str()
        .map(str::to_owned)
}

fn cargo_dependencies(value: &toml::Value) -> Vec<(&str, &toml::Value)> {
    fn append_table<'a>(
        value: Option<&'a toml::Value>,
        output: &mut Vec<(&'a str, &'a toml::Value)>,
    ) {
        if let Some(table) = value.and_then(toml::Value::as_table) {
            output.extend(table.iter().map(|(name, value)| (name.as_str(), value)));
        }
    }

    let mut dependencies = Vec::new();
    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        append_table(value.get(key), &mut dependencies);
    }
    if let Some(workspace) = value.get("workspace") {
        append_table(workspace.get("dependencies"), &mut dependencies);
    }
    if let Some(targets) = value.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
                append_table(target.get(key), &mut dependencies);
            }
        }
    }
    dependencies
}

fn nearest_workspace_dependency<'a>(
    manifests: &'a BTreeMap<String, toml::Value>,
    package_directory: &str,
    dependency: &str,
) -> Option<(String, &'a toml::Value)> {
    manifests
        .iter()
        .filter_map(|(path, value)| {
            let workspace_directory = manifest_directory(path);
            let contains_package = workspace_directory == "."
                || package_directory == workspace_directory
                || package_directory.starts_with(&format!("{workspace_directory}/"));
            contains_package
                .then(|| {
                    value
                        .get("workspace")?
                        .get("dependencies")?
                        .get(dependency)
                        .map(|value| {
                            (
                                workspace_directory.len(),
                                workspace_directory.clone(),
                                value,
                            )
                        })
                })
                .flatten()
        })
        .max_by_key(|(depth, _, _)| *depth)
        .map(|(_, directory, value)| (directory, value))
}

fn string_array(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .map(|value| value.replace('\\', "/").trim_end_matches('/').to_owned())
        .collect()
}

fn npm_workspace_patterns(value: &serde_json::Value) -> Vec<String> {
    let workspaces = value.get("workspaces");
    let packages = workspaces
        .and_then(serde_json::Value::as_array)
        .or_else(|| {
            workspaces
                .and_then(serde_json::Value::as_object)
                .and_then(|value| value.get("packages"))
                .and_then(serde_json::Value::as_array)
        });
    packages
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(|value| value.replace('\\', "/").trim_end_matches('/').to_owned())
        .collect()
}

fn npm_alias_target(requirement: &str) -> Option<String> {
    let alias = requirement.strip_prefix("npm:")?;
    if alias.starts_with('@') {
        let slash = alias.find('/')?;
        let version = alias[slash + 1..]
            .find('@')
            .map(|offset| slash + 1 + offset)
            .unwrap_or(alias.len());
        Some(alias[..version].to_owned())
    } else {
        Some(alias.split('@').next().unwrap_or(alias).to_owned())
    }
}

fn npm_local_path(requirement: &str) -> Option<&str> {
    for prefix in ["file:", "link:", "portal:"] {
        if let Some(path) = requirement.strip_prefix(prefix) {
            return Some(path);
        }
    }
    if requirement.starts_with("./") || requirement.starts_with("../") {
        Some(requirement)
    } else {
        requirement
            .strip_prefix("workspace:")
            .filter(|value| value.starts_with("./") || value.starts_with("../"))
    }
}

fn cargo_package_directory(
    path: &str,
    packages_by_dir: &BTreeMap<String, String>,
) -> Option<String> {
    let directory = if path.ends_with("/Cargo.toml") {
        manifest_directory(path)
    } else {
        path.trim_end_matches('/').to_owned()
    };
    packages_by_dir
        .contains_key(&directory)
        .then_some(directory)
}

fn npm_package_directory(path: &str, packages_by_dir: &BTreeMap<String, String>) -> Option<String> {
    let directory = if path.ends_with("/package.json") {
        manifest_directory(path)
    } else {
        path.trim_end_matches('/').to_owned()
    };
    packages_by_dir
        .contains_key(&directory)
        .then_some(directory)
}

fn local_target_paths(findings: &[AnalyzerFinding]) -> BTreeMap<String, String> {
    let mut targets = BTreeMap::new();
    for finding in findings {
        let Some(path) = finding.target_path.as_ref() else {
            continue;
        };
        if let Some(target) = finding.target.as_ref() {
            targets
                .entry(target.clone())
                .or_insert_with(|| path.clone());
            targets
                .entry(target.replace('-', "_"))
                .or_insert_with(|| path.clone());
        }
        if finding.kind == "dependency" {
            targets
                .entry(finding.label.clone())
                .or_insert_with(|| path.clone());
            targets
                .entry(finding.label.replace('-', "_"))
                .or_insert_with(|| path.clone());
        }
    }
    targets
}

fn source_capability(extension: &str) -> Option<(&'static str, bool)> {
    Some(match extension {
        "rs" => ("rust_imports", true),
        "ts" | "tsx" | "js" | "jsx" | "mts" | "cts" | "mjs" | "cjs" => ("javascript_imports", true),
        "py" | "pyi" => ("python_imports", true),
        "go" => ("go_imports", false),
        "java" => ("java_imports", false),
        "kt" | "kts" => ("kotlin_imports", false),
        "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" => ("c_cpp_imports", false),
        "cs" => ("csharp_imports", false),
        "rb" => ("ruby_imports", false),
        "php" => ("php_imports", false),
        "swift" => ("swift_imports", false),
        "scala" => ("scala_imports", false),
        "dart" => ("dart_imports", false),
        "ex" | "exs" => ("elixir_imports", false),
        "lua" => ("lua_imports", false),
        "vue" => ("vue_imports", false),
        "svelte" => ("svelte_imports", false),
        "m" | "mm" => ("objective_c_imports", false),
        "fs" | "fsx" => ("fsharp_imports", false),
        "hs" | "lhs" => ("haskell_imports", false),
        "zig" => ("zig_imports", false),
        "sh" | "bash" | "zsh" => ("shell_imports", false),
        "r" => ("r_imports", false),
        "groovy" => ("groovy_imports", false),
        "sol" => ("solidity_imports", false),
        "pl" | "pm" => ("perl_imports", false),
        "ml" | "mli" => ("ocaml_imports", false),
        "clj" | "cljs" => ("clojure_imports", false),
        "erl" | "hrl" => ("erlang_imports", false),
        _ => return None,
    })
}

fn resolve_static_import(
    root: &Path,
    source_path: &str,
    analyzer: &str,
    line: &str,
    local_targets: &BTreeMap<String, String>,
) -> Option<(String, Option<String>)> {
    match analyzer {
        "javascript_imports" => {
            let specifier = javascript_import_target(line)?;
            if specifier.starts_with('.') {
                let base = manifest_directory(source_path);
                let candidate = resolve_relative_path(&base, &specifier)?;
                let target_path = resolve_source_file(
                    root,
                    &candidate,
                    &["ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs", "json"],
                );
                Some((specifier, target_path))
            } else {
                let package = javascript_package_name(&specifier);
                let target_path = local_targets
                    .get(&package)
                    .cloned()
                    .and_then(|package_path| {
                        let suffix = specifier
                            .strip_prefix(&package)
                            .unwrap_or("")
                            .trim_start_matches('/');
                        if suffix.is_empty() {
                            Some(package_path)
                        } else {
                            let candidate = resolve_relative_path(&package_path, suffix)?;
                            resolve_source_file(
                                root,
                                &candidate,
                                &["ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs", "json"],
                            )
                            .or(Some(package_path))
                        }
                    });
                Some((package, target_path))
            }
        }
        "rust_imports" => {
            let (target, is_module_declaration) = rust_import_target(line)?;
            let first = target
                .trim_start_matches("::")
                .split("::")
                .next()
                .unwrap_or("")
                .trim_start_matches("r#");
            let target_path =
                if matches!(first, "crate" | "self" | "super") || is_module_declaration {
                    resolve_rust_target(root, source_path, &target, is_module_declaration)
                } else {
                    local_targets.get(first).cloned()
                };
            Some((target, target_path))
        }
        "python_imports" => {
            let target = python_import_target(line)?;
            let target_path = resolve_python_target(root, source_path, &target);
            Some((target, target_path))
        }
        _ => None,
    }
}

fn javascript_import_target(line: &str) -> Option<String> {
    let line = line.trim();
    let relevant = if line.starts_with("import ") && !line.starts_with("import(") {
        line
    } else if line.starts_with("export ") && line.contains(" from ") {
        line.split_once(" from ").map(|(_, value)| value)?
    } else if line.contains("require(") {
        line.split_once("require(").map(|(_, value)| value)?
    } else {
        return None;
    };
    first_quoted(relevant).map(str::to_owned)
}

fn javascript_package_name(specifier: &str) -> String {
    let specifier = specifier.split(['?', '#']).next().unwrap_or(specifier);
    if specifier.starts_with('@') {
        specifier.split('/').take(2).collect::<Vec<_>>().join("/")
    } else {
        specifier.split('/').next().unwrap_or(specifier).to_owned()
    }
}

fn rust_import_target(line: &str) -> Option<(String, bool)> {
    let mut line = line.trim();
    if let Some(visible) = line.strip_prefix("pub ") {
        line = visible;
    } else if let Some(restricted) = line.strip_prefix("pub(") {
        line = restricted.split_once(") ")?.1;
    }
    if let Some(module) = line.strip_prefix("mod ") {
        let module = module
            .trim_end_matches(';')
            .split_whitespace()
            .next()?
            .trim_start_matches("r#");
        return (!module.is_empty()).then(|| (module.to_owned(), true));
    }
    let import = line.strip_prefix("use ")?.trim_end_matches(';').trim();
    let import = import.split(" as ").next().unwrap_or(import).trim();
    let import = import
        .split('{')
        .next()
        .unwrap_or(import)
        .trim_end_matches(':')
        .trim();
    (!import.is_empty()).then(|| (import.to_owned(), false))
}

fn python_import_target(line: &str) -> Option<String> {
    let line = line.trim();
    if let Some(import) = line.strip_prefix("import ") {
        let target = import.split(',').next()?.split_whitespace().next()?.trim();
        return (!target.is_empty()).then(|| target.to_owned());
    }
    let target = line.strip_prefix("from ")?.split_whitespace().next()?;
    (!target.is_empty()).then(|| target.to_owned())
}

fn resolve_rust_target(
    root: &Path,
    source_path: &str,
    target: &str,
    is_module_declaration: bool,
) -> Option<String> {
    let source = Path::new(source_path);
    let source_dir = manifest_directory(source_path);
    let mut segments: Vec<&str> = target
        .trim_start_matches("::")
        .split("::")
        .map(|value| value.trim_start_matches("r#"))
        .filter(|value| !value.is_empty())
        .collect();
    let base = if is_module_declaration {
        let stem = source
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if matches!(stem, "lib" | "main" | "mod") {
            source_dir
        } else {
            resolve_relative_path(&source_dir, stem)?
        }
    } else {
        match segments.first().copied()? {
            "crate" => {
                segments.remove(0);
                nearest_cargo_source_root(root, source_path)?
            }
            "self" => {
                segments.remove(0);
                source_dir
            }
            "super" => {
                let mut base = source_dir;
                while segments.first() == Some(&"super") {
                    segments.remove(0);
                    base = manifest_directory(&base);
                }
                base
            }
            _ => return None,
        }
    };
    while !segments.is_empty() {
        let candidate = resolve_relative_path(&base, &segments.join("/"))?;
        if let Some(path) = resolve_source_file(root, &candidate, &["rs"]) {
            return Some(path);
        }
        segments.pop();
    }
    resolve_source_file(root, &base, &["rs"])
}

fn nearest_cargo_source_root(root: &Path, source_path: &str) -> Option<String> {
    let mut directory = Path::new(source_path).parent()?;
    loop {
        let relative = relative_display(directory);
        if root.join(&relative).join("Cargo.toml").is_file() {
            return resolve_relative_path(&relative, "src");
        }
        directory = directory.parent()?;
    }
}

fn resolve_python_target(root: &Path, source_path: &str, target: &str) -> Option<String> {
    let leading = target.chars().take_while(|value| *value == '.').count();
    let module = target.trim_start_matches('.').replace('.', "/");
    let base = if leading == 0 {
        ".".to_owned()
    } else {
        let mut base = manifest_directory(source_path);
        for _ in 1..leading {
            base = manifest_directory(&base);
        }
        base
    };
    let candidate = resolve_relative_path(&base, &module)?;
    resolve_source_file(root, &candidate, &["py", "pyi"])
}

fn resolve_source_file(root: &Path, candidate: &str, extensions: &[&str]) -> Option<String> {
    let candidate = candidate.trim_end_matches('/');
    if root.join(candidate).is_file() {
        return Some(candidate.to_owned());
    }
    for extension in extensions {
        let file = format!("{candidate}.{extension}");
        if root.join(&file).is_file() {
            return Some(file);
        }
    }
    for extension in extensions {
        let index = format!("{candidate}/index.{extension}");
        if root.join(&index).is_file() {
            return Some(index);
        }
        let module = format!("{candidate}/mod.{extension}");
        if root.join(&module).is_file() {
            return Some(module);
        }
        let init = format!("{candidate}/__init__.{extension}");
        if root.join(&init).is_file() {
            return Some(init);
        }
    }
    None
}

#[cfg(test)]
fn static_import_line(analyzer: &str, line: &str) -> bool {
    match analyzer {
        "rust_imports" => rust_import_target(line).is_some(),
        "javascript_imports" => javascript_import_target(line).is_some(),
        "python_imports" => python_import_target(line).is_some(),
        _ => false,
    }
}

fn first_quoted(value: &str) -> Option<&str> {
    let start = value.find(['\'', '"'])?;
    let quote = value.as_bytes()[start] as char;
    let rest = &value[start + 1..];
    let end = rest.find(quote)?;
    Some(&rest[..end])
}

fn project_files(root: &Path) -> Vec<String> {
    let mut files = BTreeSet::new();
    if let Some(listing) = git_output(
        root,
        &["ls-files", "--cached", "--others", "--exclude-standard"],
    ) {
        for path in listing.lines().map(|path| path.replace('\\', "/")) {
            if !ignored_project_path(&path) && root.join(&path).is_file() {
                files.insert(path);
            }
        }
    }
    if files.is_empty() {
        collect_project_files(root, root, &mut files);
    }
    files.into_iter().collect()
}

fn ignored_project_path(path: &str) -> bool {
    Path::new(path).components().any(|component| {
        matches!(component, std::path::Component::Normal(value) if IGNORED_DIRS.contains(&value.to_string_lossy().as_ref()))
    })
}

fn collect_project_files(root: &Path, directory: &Path, files: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            let name = entry.file_name();
            if IGNORED_DIRS.contains(&name.to_string_lossy().as_ref()) {
                continue;
            }
            collect_project_files(root, &path, files);
        } else if metadata.is_file() {
            if let Ok(relative) = path.strip_prefix(root) {
                files.insert(relative_display(relative));
            }
        }
    }
}

fn manifest_directory(manifest: &str) -> String {
    Path::new(manifest)
        .parent()
        .map(relative_display)
        .unwrap_or_else(|| ".".to_owned())
}

fn project_label(root: &Path, directory: &str) -> String {
    if directory == "." {
        root.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("project")
            .to_owned()
    } else {
        Path::new(directory)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("package")
            .to_owned()
    }
}

fn relative_to_directory(base: &str, path: &str) -> String {
    if base == "." {
        path.to_owned()
    } else {
        path.strip_prefix(&format!("{base}/"))
            .unwrap_or(path)
            .to_owned()
    }
}

fn resolve_relative_path(base: &str, value: &str) -> Option<String> {
    let value = value.split(['?', '#']).next().unwrap_or(value);
    let path = if base == "." {
        PathBuf::from(value)
    } else {
        Path::new(base).join(value)
    };
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
            std::path::Component::ParentDir => {
                parts.pop()?;
            }
            std::path::Component::Prefix(_) | std::path::Component::RootDir => return None,
        }
    }
    Some(if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    })
}

fn matches_any_path_pattern(patterns: &[String], path: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| path_pattern_matches(pattern, path))
}

fn path_pattern_matches(pattern: &str, path: &str) -> bool {
    fn segment_matches(pattern: &str, value: &str) -> bool {
        let pattern: Vec<char> = pattern.chars().collect();
        let value: Vec<char> = value.chars().collect();
        let (mut p, mut v, mut star, mut retry) = (0, 0, None, 0);
        while v < value.len() {
            if p < pattern.len() && (pattern[p] == '?' || pattern[p] == value[v]) {
                p += 1;
                v += 1;
            } else if p < pattern.len() && pattern[p] == '*' {
                star = Some(p);
                p += 1;
                retry = v;
            } else if let Some(star_index) = star {
                p = star_index + 1;
                retry += 1;
                v = retry;
            } else {
                return false;
            }
        }
        while p < pattern.len() && pattern[p] == '*' {
            p += 1;
        }
        p == pattern.len()
    }

    fn matches(pattern: &[&str], path: &[&str]) -> bool {
        if pattern.is_empty() {
            return path.is_empty();
        }
        if pattern[0] == "**" {
            return matches(&pattern[1..], path)
                || (!path.is_empty() && matches(pattern, &path[1..]));
        }
        !path.is_empty()
            && segment_matches(pattern[0], path[0])
            && matches(&pattern[1..], &path[1..])
    }

    let pattern = pattern.trim_start_matches("./").trim_end_matches('/');
    let path = path.trim_start_matches("./").trim_end_matches('/');
    matches(
        &pattern
            .split('/')
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>(),
        &path
            .split('/')
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>(),
    )
}

fn is_under_path(path: &str, directory: &str) -> bool {
    path == directory || path.starts_with(&format!("{directory}/"))
}

fn document_label(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path))
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                let heading = line
                    .trim()
                    .strip_prefix('#')?
                    .trim_start_matches('#')
                    .trim();
                (!heading.is_empty()).then(|| heading.chars().take(160).collect())
            })
        })
        .unwrap_or_else(|| {
            Path::new(path)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Architecture document")
                .to_owned()
        })
}

fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn git_status(root: &Path) -> String {
    git_output(root, &["status", "--porcelain=v1", "--untracked-files=all"])
        .unwrap_or_default()
        .lines()
        .filter(|line| {
            line.get(3..)
                .and_then(|path| path.split(" -> ").last())
                .is_none_or(|path| !ignored_project_path(path))
        })
        .collect::<Vec<_>>()
        .join("\n")
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
    project_files(root).len()
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

    fn write(root: &Path, path: &str, content: &str) {
        let destination = root.join(path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(destination, content).unwrap();
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
    fn persists_reuses_and_invalidates_the_default_json_project_model() {
        let root = project();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("package.json"),
            r#"{"name":"json-index-fixture"}"#,
        )
        .unwrap();
        fs::write(root.join("src/index.ts"), "export const first = 1;\n").unwrap();
        silent_command("git")
            .arg("-C")
            .arg(&root)
            .arg("init")
            .output()
            .unwrap();

        let service = ProjectIndexService::open(&root).unwrap();
        let first = service.current_snapshot().unwrap();
        let destination = service.snapshot_path();
        assert_eq!(
            destination
                .strip_prefix(root.canonicalize().unwrap())
                .unwrap(),
            Path::new(PROJECT_MODEL_INDEX_PATH)
        );
        let persisted: ProjectModelSnapshot =
            serde_json::from_slice(&fs::read(&destination).unwrap()).unwrap();
        assert_eq!(persisted, first);

        let second = service.current_snapshot().unwrap();
        assert_eq!(second.model_version, first.model_version);
        assert!(second
            .nodes
            .iter()
            .all(|node| !node.relative_path.starts_with(".vibehub/")));

        fs::write(root.join("src/second.ts"), "export const second = 2;\n").unwrap();
        let third = service.current_snapshot().unwrap();
        assert_ne!(third.model_version, first.model_version);
        assert!(third
            .nodes
            .iter()
            .any(|node| node.relative_path == "src/second.ts"));
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
        assert!(invalidation_units("docs/v3/rfc-backlog/003-index.md")
            .contains(&InvalidationUnit::DeclaredDocumentation));
        assert!(invalidation_units("docs/project-architecture.md")
            .contains(&InvalidationUnit::DeclaredDocumentation));
        assert!(invalidation_units("cmd/server.go").contains(&InvalidationUnit::ImportsSymbols));
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

    #[test]
    fn old_analyzer_findings_deserialize_without_targets() {
        let finding: AnalyzerFinding = serde_json::from_str(
            r#"{"analyzer":"cargo","version":"1","kind":"manifest","path":"Cargo.toml","label":"demo"}"#,
        )
        .unwrap();
        assert_eq!(finding.target, None);
        assert_eq!(finding.target_path, None);
    }

    #[test]
    fn models_mixed_root_manifests_and_cargo_alias_path_dependencies() {
        let root = project();
        write(
            &root,
            "Cargo.toml",
            r#"
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
core_alias = { package = "core-real", path = "crates/core" }
"#,
        );
        write(
            &root,
            "package.json",
            r#"{"name":"web-root","private":true}"#,
        );
        write(
            &root,
            "crates/core/Cargo.toml",
            r#"[package]
name = "core-real"
version = "0.1.0"
"#,
        );
        write(
            &root,
            "crates/cli/Cargo.toml",
            r#"[package]
name = "demo-cli"
version = "0.1.0"

[dependencies]
core_alias = { workspace = true }
"#,
        );

        let findings = analyze_project(&root);
        assert!(findings.iter().any(|finding| {
            finding.analyzer == "cargo"
                && finding.kind == "workspace_manifest"
                && finding.path == "Cargo.toml"
                && finding.target_path.as_deref() == Some(".")
        }));
        assert!(findings.iter().any(|finding| {
            finding.analyzer == "npm"
                && finding.kind == "package_manifest"
                && finding.target.as_deref() == Some("web-root")
                && finding.target_path.as_deref() == Some(".")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == "workspace_member"
                && finding.target.as_deref() == Some("core-real")
                && finding.target_path.as_deref() == Some("crates/core")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == "dependency"
                && finding.path == "crates/cli/Cargo.toml"
                && finding.label == "core_alias"
                && finding.target.as_deref() == Some("core-real")
                && finding.target_path.as_deref() == Some("crates/core")
        }));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn models_npm_workspace_and_workspace_or_file_dependencies() {
        let root = project();
        write(
            &root,
            "package.json",
            r#"{
  "name": "monorepo",
  "private": true,
  "workspaces": ["packages/*"],
  "dependencies": {"@demo/ui": "workspace:*"}
}"#,
        );
        write(
            &root,
            "packages/ui/package.json",
            r#"{"name":"@demo/ui","version":"1.0.0"}"#,
        );
        write(
            &root,
            "packages/web/package.json",
            r#"{
  "name": "@demo/web",
  "version": "1.0.0",
  "dependencies": {"ui-local": "file:../ui"}
}"#,
        );

        let findings = analyze_project(&root);
        assert!(findings.iter().any(|finding| {
            finding.kind == "workspace_member"
                && finding.target.as_deref() == Some("@demo/ui")
                && finding.target_path.as_deref() == Some("packages/ui")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == "dependency"
                && finding.path == "package.json"
                && finding.target.as_deref() == Some("@demo/ui")
                && finding.target_path.as_deref() == Some("packages/ui")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == "dependency"
                && finding.path == "packages/web/package.json"
                && finding.label == "ui-local"
                && finding.target.as_deref() == Some("@demo/ui")
                && finding.target_path.as_deref() == Some("packages/ui")
        }));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn resolves_javascript_rust_and_python_static_import_targets() {
        let root = project();
        write(
            &root,
            "Cargo.toml",
            r#"[package]
name = "demo"
version = "0.1.0"
"#,
        );
        write(
            &root,
            "package.json",
            r#"{"name":"frontend","workspaces":["packages/*"]}"#,
        );
        write(&root, "packages/ui/package.json", r#"{"name":"@demo/ui"}"#);
        write(&root, "packages/ui/button.ts", "export const button = 1;");
        write(
            &root,
            "frontend/main.ts",
            "import { helper } from './helper';\nimport { button } from '@demo/ui/button';\n",
        );
        write(&root, "frontend/helper.ts", "export const helper = 1;");
        write(
            &root,
            "src/lib.rs",
            "mod model;\nuse crate::model::Model;\n",
        );
        write(&root, "src/model.rs", "pub struct Model;");
        write(&root, "python/main.py", "from app.model import Model\n");
        write(&root, "app/model.py", "class Model: pass\n");

        let findings = analyze_project(&root);
        assert!(findings.iter().any(|finding| {
            finding.kind == "static_import"
                && finding.path == "frontend/main.ts"
                && finding.target.as_deref() == Some("./helper")
                && finding.target_path.as_deref() == Some("frontend/helper.ts")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == "static_import"
                && finding.target.as_deref() == Some("@demo/ui")
                && finding.target_path.as_deref() == Some("packages/ui/button.ts")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == "static_import"
                && finding.path == "src/lib.rs"
                && finding.target_path.as_deref() == Some("src/model.rs")
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == "static_import"
                && finding.path == "python/main.py"
                && finding.target.as_deref() == Some("app.model")
                && finding.target_path.as_deref() == Some("app/model.py")
        }));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn records_declared_architecture_documents() {
        let root = project();
        write(&root, "README.md", "# Demo\n");
        write(&root, "docs/adr/0001-store.md", "# Store decision\n");
        write(&root, "docs/rfc/0002-api.md", "# API RFC\n");
        write(&root, "docs/v3/rfc-backlog/0003-index.md", "# Index RFC\n");
        write(
            &root,
            "docs/project-architecture.md",
            "# Project architecture\n",
        );
        write(&root, "docs/product-redesign.md", "# Product redesign\n");

        let findings = analyze_project(&root);
        assert!(findings
            .iter()
            .any(|finding| finding.kind == "readme" && finding.path == "README.md"));
        for path in [
            "docs/adr/0001-store.md",
            "docs/rfc/0002-api.md",
            "docs/v3/rfc-backlog/0003-index.md",
            "docs/project-architecture.md",
            "docs/product-redesign.md",
        ] {
            assert!(findings.iter().any(|finding| {
                finding.kind == "architecture_document"
                    && finding.path == path
                    && finding.target_path.as_deref() == Some(path)
            }));
        }
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn reports_only_source_capabilities_that_are_present() {
        let root = project();
        write(&root, "cmd/main.go", "package main\n");

        let findings = analyze_project(&root);
        assert!(findings.iter().any(|finding| {
            finding.kind == "capability"
                && finding.analyzer == "go_imports"
                && finding.target.as_deref() == Some("unsupported")
        }));
        assert!(!findings
            .iter()
            .any(|finding| finding.analyzer == "python_imports"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn manifest_parse_errors_degrade_explicitly() {
        let root = project();
        write(&root, "Cargo.toml", "[workspace\n");
        write(&root, "package.json", "{not-json}");

        let findings = analyze_project(&root);
        assert!(findings.iter().any(|finding| {
            finding.kind == "analyzer_error"
                && finding.analyzer == "cargo"
                && finding.path == "Cargo.toml"
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == "analyzer_error"
                && finding.analyzer == "npm"
                && finding.path == "package.json"
        }));
        assert!(!findings.iter().any(|finding| {
            finding.kind == "workspace_manifest" || finding.kind == "package_manifest"
        }));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn analyzer_findings_are_sorted_deduplicated_and_deterministic() {
        let root = project();
        write(
            &root,
            "Cargo.toml",
            r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
serde = "1"

[dev-dependencies]
serde = "1"
"#,
        );
        write(
            &root,
            "src/lib.rs",
            "use serde::Serialize;\nuse serde::Serialize;\n",
        );

        let first = analyze_project(&root);
        let second = analyze_project(&root);
        assert_eq!(first, second);
        assert!(first.windows(2).all(|items| items[0] < items[1]));
        assert_eq!(
            first
                .iter()
                .filter(|finding| {
                    finding.kind == "dependency"
                        && finding.path == "Cargo.toml"
                        && finding.label == "serde"
                })
                .count(),
            1
        );
        fs::remove_dir_all(root).ok();
    }
}
