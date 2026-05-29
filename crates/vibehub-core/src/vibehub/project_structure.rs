use crate::process_util::silent_command;
use crate::vibehub::util::{canonical_project_root, normalize_path, relative_to_project};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_TREE_DEPTH: usize = 3;
const MAX_CHILDREN_PER_DIR: usize = 12;
const MAX_GRAPH_DEPTH: usize = 2;
const MAX_GRAPH_NODES: usize = 48;

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
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectStructureViewData {
    pub root_path: String,
    pub source: String,
    pub semantic_graph_available: bool,
    pub graph_nodes: Vec<ProjectStructureGraphNode>,
    pub graph_edges: Vec<ProjectStructureGraphEdge>,
    pub tree: Vec<ProjectStructureTreeNode>,
    pub scanned_files_count: usize,
    pub scanned_dirs_count: usize,
    pub truncated: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectStructureGraphNode {
    pub id: String,
    pub label: String,
    pub path: String,
    pub kind: String,
    pub depth: usize,
    pub changed: bool,
    pub file_count: usize,
    pub directory_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectStructureGraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectStructureTreeNode {
    pub id: String,
    pub label: String,
    pub path: String,
    pub kind: String,
    pub depth: usize,
    pub changed: bool,
    pub file_count: usize,
    pub directory_count: usize,
    pub children: Vec<ProjectStructureTreeNode>,
    pub truncated: bool,
}

#[derive(Debug)]
struct ScanState {
    changed_files: BTreeSet<String>,
    scanned_files_count: usize,
    scanned_dirs_count: usize,
    truncated: bool,
    warnings: Vec<String>,
}

pub fn read_project_structure(project_root: impl AsRef<Path>) -> Result<ProjectStructureViewData> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let changed_files = git_changed_files(&project_root);
    let mut state = ScanState {
        changed_files,
        scanned_files_count: 0,
        scanned_dirs_count: 0,
        truncated: false,
        warnings: Vec::new(),
    };

    let root = scan_tree_node(&project_root, Path::new("."), 0, &mut state).with_context(|| {
        format!(
            "Failed to scan project structure for {}",
            project_root.display()
        )
    })?;
    let mut graph_nodes = Vec::new();
    let mut graph_edges = Vec::new();
    collect_graph(&root, None, &mut graph_nodes, &mut graph_edges);

    Ok(ProjectStructureViewData {
        root_path: normalize_path(&project_root),
        source: "filesystem_scan".to_string(),
        semantic_graph_available: false,
        graph_nodes,
        graph_edges,
        tree: root.children,
        scanned_files_count: state.scanned_files_count,
        scanned_dirs_count: state.scanned_dirs_count,
        truncated: state.truncated,
        warnings: state.warnings,
    })
}

pub fn resolve_project_file_path(
    project_root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let rel = relative_path.as_ref();

    if rel.is_absolute()
        || rel.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(anyhow!("Path escapes project root: {}", rel.display()));
    }

    let full = project_root.join(rel);
    let canonical_root = fs::canonicalize(&project_root)
        .with_context(|| format!("Failed to canonicalize {}", project_root.display()))?;
    let resolved = if full.exists() {
        fs::canonicalize(&full)
            .with_context(|| format!("Failed to canonicalize {}", full.display()))?
    } else {
        let parent = full
            .parent()
            .ok_or_else(|| anyhow!("Path has no parent: {}", full.display()))?;
        let parent_canon = if parent.exists() {
            fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf())
        } else {
            parent.to_path_buf()
        };
        let file_name = full
            .file_name()
            .ok_or_else(|| anyhow!("Path has no file name: {}", full.display()))?;
        parent_canon.join(file_name)
    };

    if !resolved.starts_with(&canonical_root) {
        return Err(anyhow!(
            "Path is not within project root: {}",
            rel.display()
        ));
    }

    Ok((project_root, resolved, rel.to_path_buf()))
}

fn scan_tree_node(
    project_root: &Path,
    rel_path: &Path,
    depth: usize,
    state: &mut ScanState,
) -> Result<ProjectStructureTreeNode> {
    let full_path = project_root.join(rel_path);
    let metadata = fs::symlink_metadata(&full_path)
        .with_context(|| format!("Failed to read metadata for {}", full_path.display()))?;
    let file_type = metadata.file_type();
    let is_dir = file_type.is_dir() && !file_type.is_symlink();
    let kind = if depth == 0 {
        "root"
    } else if is_dir {
        "directory"
    } else {
        "file"
    };
    let path = normalize_rel(rel_path);
    let changed = is_changed(&path, is_dir, &state.changed_files);
    let label = if depth == 0 {
        project_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("project")
            .to_string()
    } else {
        rel_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string()
    };

    if is_dir {
        state.scanned_dirs_count += 1;
    } else {
        state.scanned_files_count += 1;
    }

    let mut children = Vec::new();
    let mut truncated = false;
    if is_dir {
        if depth >= MAX_TREE_DEPTH {
            if has_visible_children(&full_path) {
                truncated = true;
                state.truncated = true;
            }
        } else {
            let entries = read_visible_entries(&full_path)?;
            if entries.len() > MAX_CHILDREN_PER_DIR {
                truncated = true;
                state.truncated = true;
                state.warnings.push(format!(
                    "{} contains {} items; showing first {}",
                    path,
                    entries.len(),
                    MAX_CHILDREN_PER_DIR
                ));
            }
            for entry in entries.into_iter().take(MAX_CHILDREN_PER_DIR) {
                let child_rel = relative_to_project(project_root, &entry).unwrap_or_else(|_| {
                    entry
                        .file_name()
                        .map(|name| rel_path.join(name))
                        .unwrap_or_else(|| rel_path.to_path_buf())
                });
                if let Ok(child) = scan_tree_node(project_root, &child_rel, depth + 1, state) {
                    children.push(child);
                }
            }
        }
    }

    let file_count =
        children.iter().map(|child| child.file_count).sum::<usize>() + usize::from(kind == "file");
    let directory_count = children
        .iter()
        .map(|child| child.directory_count)
        .sum::<usize>()
        + usize::from(kind == "directory" || kind == "root");

    Ok(ProjectStructureTreeNode {
        id: node_id(&path),
        label,
        path,
        kind: kind.to_string(),
        depth,
        changed,
        file_count,
        directory_count,
        children,
        truncated,
    })
}

fn collect_graph(
    node: &ProjectStructureTreeNode,
    parent_id: Option<&str>,
    nodes: &mut Vec<ProjectStructureGraphNode>,
    edges: &mut Vec<ProjectStructureGraphEdge>,
) {
    if node.depth > MAX_GRAPH_DEPTH || nodes.len() >= MAX_GRAPH_NODES {
        return;
    }

    nodes.push(ProjectStructureGraphNode {
        id: node.id.clone(),
        label: node.label.clone(),
        path: node.path.clone(),
        kind: node.kind.clone(),
        depth: node.depth,
        changed: node.changed,
        file_count: node.file_count,
        directory_count: node.directory_count,
    });

    if let Some(from) = parent_id {
        edges.push(ProjectStructureGraphEdge {
            from: from.to_string(),
            to: node.id.clone(),
            kind: "contains".to_string(),
        });
    }

    for child in &node.children {
        collect_graph(child, Some(&node.id), nodes, edges);
    }
}

fn read_visible_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("Failed to read {}", dir.display()))? {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if should_ignore(&name, &path) {
            continue;
        }
        entries.push(path);
    }
    entries.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
    Ok(entries)
}

fn has_visible_children(dir: &Path) -> bool {
    read_visible_entries(dir)
        .map(|entries| !entries.is_empty())
        .unwrap_or(false)
}

fn should_ignore(name: &str, path: &Path) -> bool {
    if IGNORED_DIRS.contains(&name) {
        return true;
    }
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn sort_key(path: &Path) -> (u8, String) {
    let is_file = path.is_file();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    (u8::from(is_file), name)
}

fn git_changed_files(project_root: &Path) -> BTreeSet<String> {
    if !is_git_repo(project_root) {
        return BTreeSet::new();
    }
    let mut changed = git_lines(project_root, &["diff", "--name-only", "HEAD", "--"]);
    changed.extend(git_lines(
        project_root,
        &["ls-files", "--others", "--exclude-standard"],
    ));
    changed.into_iter().collect()
}

fn is_git_repo(project_root: &Path) -> bool {
    silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_lines(project_root: &Path, args: &[&str]) -> Vec<String> {
    let Ok(output) = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.replace('\\', "/"))
        .collect()
}

fn is_changed(path: &str, is_dir: bool, changed_files: &BTreeSet<String>) -> bool {
    if path == "." {
        return !changed_files.is_empty();
    }
    changed_files.contains(path)
        || (is_dir
            && changed_files
                .iter()
                .any(|changed| changed.starts_with(&format!("{path}/"))))
}

fn normalize_rel(path: &Path) -> String {
    let normalized = normalize_path(path);
    if normalized.is_empty() {
        ".".to_string()
    } else {
        normalized
    }
}

fn node_id(path: &str) -> String {
    if path == "." {
        "root".to_string()
    } else {
        format!("node:{}", path.replace('/', ":"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let p = std::env::temp_dir().join(format!("vibehub-structure-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&p).expect("create");
        p
    }

    fn flatten_paths(nodes: &[ProjectStructureTreeNode], out: &mut Vec<String>) {
        for node in nodes {
            out.push(node.path.clone());
            flatten_paths(&node.children, out);
        }
    }

    #[test]
    fn scans_project_structure_without_heavy_dirs() {
        let project = temp_project();
        fs::create_dir_all(project.join("src/components")).expect("src");
        fs::create_dir_all(project.join("src-tauri/src")).expect("tauri");
        fs::create_dir_all(project.join("node_modules/pkg")).expect("node_modules");
        fs::create_dir_all(project.join("target/debug")).expect("target");
        fs::write(project.join("src/main.tsx"), "export {};").expect("main");
        fs::write(project.join("src/components/App.tsx"), "export {};").expect("app");
        fs::write(project.join("src-tauri/src/main.rs"), "fn main() {}").expect("rust main");
        fs::write(project.join("node_modules/pkg/index.js"), "").expect("pkg");
        fs::write(project.join("target/debug/app"), "").expect("target file");

        let view = read_project_structure(&project).expect("structure");
        let mut paths = Vec::new();
        flatten_paths(&view.tree, &mut paths);

        assert!(paths.contains(&"src".to_string()));
        assert!(paths.contains(&"src/components/App.tsx".to_string()));
        assert!(paths.contains(&"src-tauri/src/main.rs".to_string()));
        assert!(!paths.iter().any(|path| path.starts_with("node_modules")));
        assert!(!paths.iter().any(|path| path.starts_with("target")));
        assert!(view.graph_nodes.iter().any(|node| node.path == "src"));
        assert!(view
            .graph_edges
            .iter()
            .any(|edge| edge.from == "root" && edge.to == "node:src"));

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn project_file_resolution_rejects_parent_escape() {
        let project = temp_project();
        let err = resolve_project_file_path(&project, "../outside.txt").expect_err("reject");
        assert!(err.to_string().contains("escapes project root"));
        fs::remove_dir_all(project).ok();
    }
}
