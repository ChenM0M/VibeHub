use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

const RECENT_LIMIT: usize = 8;

#[derive(Debug, Clone, Serialize)]
pub struct LocalAgentUsageOverview {
    pub project_path: String,
    pub generated_at: String,
    pub non_cached_total_tokens: u64,
    pub total_tokens: u64,
    pub source_count: usize,
    pub codex: AgentUsageSourceSummary,
    pub opencode: AgentUsageSourceSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentUsageSourceSummary {
    pub source: String,
    pub available: bool,
    pub data_path: Option<String>,
    pub records: usize,
    pub non_cached_total_tokens: u64,
    pub total_tokens: u64,
    pub cost: Option<f64>,
    pub tokens: TokenBreakdown,
    pub latest_updated_at_ms: Option<i64>,
    pub recent: Vec<AgentUsageRecentItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TokenBreakdown {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cached_input: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentUsageRecentItem {
    pub id: String,
    pub title: String,
    pub model: Option<String>,
    pub agent: Option<String>,
    pub non_cached_total_tokens: u64,
    pub total_tokens: u64,
    pub cost: Option<f64>,
    pub updated_at_ms: Option<i64>,
}

impl TokenBreakdown {
    fn non_cached_total(&self) -> u64 {
        self.total
            .saturating_sub(self.cached_input)
            .saturating_sub(self.cache_read)
            .saturating_sub(self.cache_write)
    }
}

#[derive(Debug)]
struct CodexThreadRow {
    id: String,
    rollout_path: String,
    title: String,
    model_provider: Option<String>,
    model: Option<String>,
    tokens_used: u64,
    updated_at_ms: Option<i64>,
}

#[derive(Debug)]
struct OpenCodeSessionRow {
    id: String,
    title: String,
    agent: Option<String>,
    model: Option<String>,
    cost: f64,
    tokens_input: u64,
    tokens_output: u64,
    tokens_reasoning: u64,
    tokens_cache_read: u64,
    tokens_cache_write: u64,
    time_updated: Option<i64>,
}

#[derive(Debug)]
struct MatchedRows<T> {
    exact: Vec<T>,
    aliases: BTreeMap<String, Vec<T>>,
}

impl<T> Default for MatchedRows<T> {
    fn default() -> Self {
        Self {
            exact: Vec::new(),
            aliases: BTreeMap::new(),
        }
    }
}

pub fn read_local_agent_usage(project_path: impl AsRef<Path>) -> Result<LocalAgentUsageOverview> {
    let project_path = ProjectPathMatcher::new(project_path.as_ref());
    let mut warnings = Vec::new();
    let codex = read_codex_usage(&project_path);
    let opencode = read_opencode_usage(&project_path);

    if !codex.available {
        warnings.push(
            "Codex local usage source is unavailable or has no matching records.".to_string(),
        );
    }
    if !opencode.available {
        warnings.push(
            "OpenCode local usage source is unavailable or has no matching records.".to_string(),
        );
    }

    Ok(LocalAgentUsageOverview {
        non_cached_total_tokens: codex
            .non_cached_total_tokens
            .saturating_add(opencode.non_cached_total_tokens),
        total_tokens: codex.total_tokens.saturating_add(opencode.total_tokens),
        source_count: [codex.available, opencode.available]
            .into_iter()
            .filter(|available| *available)
            .count(),
        project_path: project_path.display_path.clone(),
        generated_at: Utc::now().to_rfc3339(),
        codex,
        opencode,
        warnings,
    })
}

#[derive(Debug, Clone)]
struct ProjectPathMatcher {
    display_path: String,
    primary: String,
    primary_children: String,
    canonical: String,
    canonical_children: String,
    basename: Option<String>,
}

impl ProjectPathMatcher {
    fn new(path: &Path) -> Self {
        let primary = normalize_path_string(path);
        let canonical = path
            .canonicalize()
            .ok()
            .map(|path| normalize_path_string(&path))
            .unwrap_or_else(|| primary.clone());
        let basename = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(|name| name.to_ascii_lowercase());

        Self {
            display_path: primary.clone(),
            primary_children: child_path_pattern(&primary),
            canonical_children: child_path_pattern(&canonical),
            primary,
            canonical,
            basename,
        }
    }

    fn matches(&self, value: &str) -> bool {
        let value = normalize_stored_path(value);
        value.eq_ignore_ascii_case(&self.primary)
            || value.eq_ignore_ascii_case(&self.canonical)
            || lower_path_starts_with(&value, &self.primary_children)
            || lower_path_starts_with(&value, &self.canonical_children)
    }

    fn alias_root(&self, value: &str) -> Option<String> {
        let basename = self.basename.as_deref()?;
        let value = normalize_stored_path(value);
        let parts = value.split('/').collect::<Vec<_>>();
        for index in (0..parts.len()).rev() {
            if parts[index].eq_ignore_ascii_case(basename) {
                return Some(parts[..=index].join("/"));
            }
        }
        None
    }
}

fn read_codex_usage(project_path: &ProjectPathMatcher) -> AgentUsageSourceSummary {
    let mut warnings = Vec::new();
    let db_paths = existing_paths(codex_state_db_candidates());
    if db_paths.is_empty() {
        return empty_source(
            "codex",
            "Codex state database was not found in CODEX_SQLITE_HOME, CODEX_HOME, or ~/.codex.",
        );
    };

    let result = read_codex_usage_from_dbs(project_path, &db_paths, &mut warnings);
    match result {
        Ok(mut summary) => {
            summary.warnings.append(&mut warnings);
            summary
        }
        Err(error) => {
            let mut summary = error_source("codex", db_paths.first().cloned(), error);
            summary.warnings = warnings;
            summary
        }
    }
}

fn read_codex_usage_from_db(
    project_path: &str,
    db_path: &Path,
    warnings: &mut Vec<String>,
) -> Result<AgentUsageSourceSummary> {
    let matcher = ProjectPathMatcher::new(Path::new(project_path));
    read_codex_usage_from_dbs(&matcher, &[db_path.to_path_buf()], warnings)
}

fn read_codex_usage_from_dbs(
    project_path: &ProjectPathMatcher,
    db_paths: &[PathBuf],
    warnings: &mut Vec<String>,
) -> Result<AgentUsageSourceSummary> {
    let mut best_exact_match: Option<(PathBuf, Vec<CodexThreadRow>)> = None;
    let mut alias_matches = Vec::new();
    let mut errors = Vec::new();

    for db_path in db_paths {
        match read_codex_rows_from_db(project_path, db_path) {
            Ok(db_rows) => {
                if should_replace_codex_match(
                    best_exact_match.as_ref().map(|(_, rows)| rows),
                    &db_rows.exact,
                ) {
                    best_exact_match = Some((db_path.clone(), db_rows.exact));
                }
                for (alias_root, rows) in db_rows.aliases {
                    alias_matches.push((db_path.clone(), alias_root, rows));
                }
            }
            Err(error) => errors.push(format!("{}: {error:#}", db_path.display())),
        }
    }

    if best_exact_match.is_none() && alias_matches.is_empty() && errors.len() == db_paths.len() {
        anyhow::bail!("{}", errors.join("; "));
    }

    warnings.extend(errors);
    let (data_path, alias_root, mut rows) =
        choose_codex_rows(project_path, best_exact_match, alias_matches, warnings);
    rows.sort_by(|a, b| {
        b.updated_at_ms
            .cmp(&a.updated_at_ms)
            .then_with(|| b.id.cmp(&a.id))
    });

    let mut tokens = TokenBreakdown::default();
    let mut recent = Vec::new();
    let mut latest_updated_at_ms = None;

    for row in rows.iter() {
        tokens.total = tokens.total.saturating_add(row.tokens_used);
        latest_updated_at_ms = max_opt_i64(latest_updated_at_ms, row.updated_at_ms);
        let usage = read_codex_rollout_usage(&row.rollout_path, warnings);
        let non_cached_total_tokens = usage
            .as_ref()
            .map(TokenBreakdown::non_cached_total)
            .unwrap_or(row.tokens_used);
        if let Some(usage) = usage {
            tokens.input = tokens.input.saturating_add(usage.input);
            tokens.output = tokens.output.saturating_add(usage.output);
            tokens.reasoning = tokens.reasoning.saturating_add(usage.reasoning);
            tokens.cached_input = tokens.cached_input.saturating_add(usage.cached_input);
        }
        if recent.len() < RECENT_LIMIT {
            recent.push(AgentUsageRecentItem {
                id: row.id.clone(),
                title: safe_title(&row.title),
                model: row.model.clone().or_else(|| row.model_provider.clone()),
                agent: Some("Codex".to_string()),
                non_cached_total_tokens,
                total_tokens: row.tokens_used,
                cost: None,
                updated_at_ms: row.updated_at_ms,
            });
        }
    }

    if rows.is_empty() {
        warnings.push(format!(
            "No Codex records matched {} or its child paths.",
            project_path.display_path
        ));
    }

    Ok(AgentUsageSourceSummary {
        source: "codex".to_string(),
        available: !rows.is_empty(),
        data_path: Some(if rows.is_empty() {
            db_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            data_path.display().to_string()
        }),
        records: rows.len(),
        non_cached_total_tokens: tokens.non_cached_total(),
        total_tokens: tokens.total,
        cost: None,
        tokens,
        latest_updated_at_ms,
        recent,
        warnings: alias_root
            .map(|path| {
                vec![format!(
                    "No Codex records matched configured path {}; using same-name usage path {}.",
                    project_path.display_path, path
                )]
            })
            .unwrap_or_default(),
    })
}

fn read_codex_rows_from_db(
    project_path: &ProjectPathMatcher,
    db_path: &Path,
) -> Result<MatchedRows<CodexThreadRow>> {
    let conn = open_readonly(db_path)?;
    let mut stmt = conn.prepare(
        "select id, rollout_path, title, model_provider, model, tokens_used, updated_at_ms, cwd \
         from threads order by updated_at_ms desc, updated_at desc, id desc",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = MatchedRows::default();
    while let Some(row) = rows.next()? {
        let cwd: String = row.get(7)?;
        let usage_row = CodexThreadRow {
            id: row.get(0)?,
            rollout_path: row.get(1)?,
            title: row.get(2)?,
            model_provider: row.get(3).ok(),
            model: row.get(4).ok(),
            tokens_used: signed_to_u64(row.get::<_, i64>(5).unwrap_or_default()),
            updated_at_ms: row.get(6).ok(),
        };
        if project_path.matches(&cwd) {
            out.exact.push(usage_row);
        } else if let Some(alias_root) = project_path.alias_root(&cwd) {
            out.aliases.entry(alias_root).or_default().push(usage_row);
        }
    }
    Ok(out)
}

fn choose_codex_rows(
    project_path: &ProjectPathMatcher,
    exact_match: Option<(PathBuf, Vec<CodexThreadRow>)>,
    alias_matches: Vec<(PathBuf, String, Vec<CodexThreadRow>)>,
    warnings: &mut Vec<String>,
) -> (PathBuf, Option<String>, Vec<CodexThreadRow>) {
    if let Some((path, rows)) = exact_match.filter(|(_, rows)| !rows.is_empty()) {
        return (path, None, rows);
    }

    let alias_roots = alias_matches
        .iter()
        .filter(|(_, _, rows)| !rows.is_empty())
        .map(|(_, alias_root, _)| alias_root.clone())
        .collect::<BTreeSet<_>>();

    if alias_roots.len() == 1 {
        let alias_root = alias_roots.into_iter().next().unwrap_or_default();
        let mut best: Option<(PathBuf, Vec<CodexThreadRow>)> = None;
        for (path, candidate_alias, rows) in alias_matches {
            if candidate_alias == alias_root
                && should_replace_codex_match(best.as_ref().map(|(_, rows)| rows), &rows)
            {
                best = Some((path, rows));
            }
        }
        if let Some((path, rows)) = best {
            return (path, Some(alias_root), rows);
        }
    } else if alias_roots.len() > 1 {
        warnings.push(format!(
            "Multiple same-name Codex usage paths matched {}; refusing ambiguous fallback: {}.",
            project_path.display_path,
            alias_roots.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    (PathBuf::new(), None, Vec::new())
}

fn should_replace_codex_match(
    current: Option<&Vec<CodexThreadRow>>,
    candidate: &[CodexThreadRow],
) -> bool {
    let Some(current) = current else {
        return true;
    };
    candidate.len() > current.len()
        || (candidate.len() == current.len()
            && max_codex_updated_at(candidate) > max_codex_updated_at(current))
}

fn max_codex_updated_at(rows: &[CodexThreadRow]) -> Option<i64> {
    rows.iter().filter_map(|row| row.updated_at_ms).max()
}

fn read_opencode_usage(project_path: &ProjectPathMatcher) -> AgentUsageSourceSummary {
    let mut warnings = Vec::new();
    let db_paths = existing_paths(opencode_db_candidates());
    if db_paths.is_empty() {
        return empty_source(
            "opencode",
            "OpenCode database was not found in XDG data, ~/.local/share/opencode, Application Support, or AppData candidates.",
        );
    };

    let result = read_opencode_usage_from_dbs(project_path, &db_paths, &mut warnings);
    match result {
        Ok(mut summary) => {
            summary.warnings.append(&mut warnings);
            summary
        }
        Err(error) => {
            let mut summary = error_source("opencode", db_paths.first().cloned(), error);
            summary.warnings = warnings;
            summary
        }
    }
}

fn read_opencode_usage_from_db(
    project_path: &str,
    db_path: &Path,
) -> Result<AgentUsageSourceSummary> {
    let matcher = ProjectPathMatcher::new(Path::new(project_path));
    let mut warnings = Vec::new();
    read_opencode_usage_from_dbs(&matcher, &[db_path.to_path_buf()], &mut warnings)
}

fn read_opencode_usage_from_dbs(
    project_path: &ProjectPathMatcher,
    db_paths: &[PathBuf],
    warnings: &mut Vec<String>,
) -> Result<AgentUsageSourceSummary> {
    let mut best_exact_match: Option<(PathBuf, Vec<OpenCodeSessionRow>)> = None;
    let mut alias_matches = Vec::new();
    let mut errors = Vec::new();

    for db_path in db_paths {
        match read_opencode_rows_from_db(project_path, db_path) {
            Ok(db_rows) => {
                if should_replace_opencode_match(
                    best_exact_match.as_ref().map(|(_, rows)| rows),
                    &db_rows.exact,
                ) {
                    best_exact_match = Some((db_path.clone(), db_rows.exact));
                }
                for (alias_root, rows) in db_rows.aliases {
                    alias_matches.push((db_path.clone(), alias_root, rows));
                }
            }
            Err(error) => errors.push(format!("{}: {error:#}", db_path.display())),
        }
    }

    if best_exact_match.is_none() && alias_matches.is_empty() && errors.len() == db_paths.len() {
        anyhow::bail!("{}", errors.join("; "));
    }

    warnings.extend(errors);
    let (data_path, alias_root, mut rows) =
        choose_opencode_rows(project_path, best_exact_match, alias_matches, warnings);
    rows.sort_by(|a, b| {
        b.time_updated
            .cmp(&a.time_updated)
            .then_with(|| b.id.cmp(&a.id))
    });

    let mut tokens = TokenBreakdown::default();
    let mut cost = 0.0_f64;
    let mut recent = Vec::new();
    let mut latest_updated_at_ms = None;

    for row in rows.iter() {
        let total = row
            .tokens_input
            .saturating_add(row.tokens_output)
            .saturating_add(row.tokens_reasoning)
            .saturating_add(row.tokens_cache_read)
            .saturating_add(row.tokens_cache_write);
        let non_cached_total_tokens = total
            .saturating_sub(row.tokens_cache_read)
            .saturating_sub(row.tokens_cache_write);
        tokens.input = tokens.input.saturating_add(row.tokens_input);
        tokens.output = tokens.output.saturating_add(row.tokens_output);
        tokens.reasoning = tokens.reasoning.saturating_add(row.tokens_reasoning);
        tokens.cache_read = tokens.cache_read.saturating_add(row.tokens_cache_read);
        tokens.cache_write = tokens.cache_write.saturating_add(row.tokens_cache_write);
        tokens.total = tokens.total.saturating_add(total);
        cost += row.cost;
        latest_updated_at_ms = max_opt_i64(latest_updated_at_ms, row.time_updated);
        if recent.len() < RECENT_LIMIT {
            recent.push(AgentUsageRecentItem {
                id: row.id.clone(),
                title: safe_title(&row.title),
                model: row.model.as_deref().map(format_opencode_model),
                agent: row.agent.clone(),
                non_cached_total_tokens,
                total_tokens: total,
                cost: Some(row.cost),
                updated_at_ms: row.time_updated,
            });
        }
    }

    if rows.is_empty() {
        warnings.push(format!(
            "No OpenCode records matched {} or its child paths.",
            project_path.display_path
        ));
    }

    Ok(AgentUsageSourceSummary {
        source: "opencode".to_string(),
        available: !rows.is_empty(),
        data_path: Some(if rows.is_empty() {
            db_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            data_path.display().to_string()
        }),
        records: rows.len(),
        non_cached_total_tokens: tokens.non_cached_total(),
        total_tokens: tokens.total,
        cost: Some(cost),
        tokens,
        latest_updated_at_ms,
        recent,
        warnings: alias_root
            .map(|path| {
                vec![format!(
                    "No OpenCode records matched configured path {}; using same-name usage path {}.",
                    project_path.display_path, path
                )]
            })
            .unwrap_or_default(),
    })
}

fn read_opencode_rows_from_db(
    project_path: &ProjectPathMatcher,
    db_path: &Path,
) -> Result<MatchedRows<OpenCodeSessionRow>> {
    let conn = open_readonly(db_path)?;
    let mut stmt = conn.prepare(
        "select s.id, s.title, s.agent, s.model, s.cost, s.tokens_input, s.tokens_output, \
                s.tokens_reasoning, s.tokens_cache_read, s.tokens_cache_write, s.time_updated, \
                p.worktree, s.directory, s.path \
         from session s \
         left join project p on p.id = s.project_id \
         order by s.time_updated desc, s.id desc",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = MatchedRows::default();
    while let Some(row) = rows.next()? {
        let worktree: Option<String> = row.get(11).ok();
        let directory: Option<String> = row.get(12).ok();
        let path: Option<String> = row.get(13).ok();
        let paths = [worktree.as_deref(), directory.as_deref(), path.as_deref()];
        let matches = paths
            .into_iter()
            .flatten()
            .any(|path| project_path.matches(path));
        let usage_row = OpenCodeSessionRow {
            id: row.get(0)?,
            title: row.get(1)?,
            agent: row.get(2).ok(),
            model: row.get(3).ok(),
            cost: row.get::<_, f64>(4).unwrap_or_default(),
            tokens_input: signed_to_u64(row.get::<_, i64>(5).unwrap_or_default()),
            tokens_output: signed_to_u64(row.get::<_, i64>(6).unwrap_or_default()),
            tokens_reasoning: signed_to_u64(row.get::<_, i64>(7).unwrap_or_default()),
            tokens_cache_read: signed_to_u64(row.get::<_, i64>(8).unwrap_or_default()),
            tokens_cache_write: signed_to_u64(row.get::<_, i64>(9).unwrap_or_default()),
            time_updated: row.get(10).ok(),
        };
        if matches {
            out.exact.push(usage_row);
        } else if let Some(alias_root) = paths
            .into_iter()
            .flatten()
            .find_map(|path| project_path.alias_root(path))
        {
            out.aliases.entry(alias_root).or_default().push(usage_row);
        }
    }
    Ok(out)
}

fn choose_opencode_rows(
    project_path: &ProjectPathMatcher,
    exact_match: Option<(PathBuf, Vec<OpenCodeSessionRow>)>,
    alias_matches: Vec<(PathBuf, String, Vec<OpenCodeSessionRow>)>,
    warnings: &mut Vec<String>,
) -> (PathBuf, Option<String>, Vec<OpenCodeSessionRow>) {
    if let Some((path, rows)) = exact_match.filter(|(_, rows)| !rows.is_empty()) {
        return (path, None, rows);
    }

    let alias_roots = alias_matches
        .iter()
        .filter(|(_, _, rows)| !rows.is_empty())
        .map(|(_, alias_root, _)| alias_root.clone())
        .collect::<BTreeSet<_>>();

    if alias_roots.len() == 1 {
        let alias_root = alias_roots.into_iter().next().unwrap_or_default();
        let mut best: Option<(PathBuf, Vec<OpenCodeSessionRow>)> = None;
        for (path, candidate_alias, rows) in alias_matches {
            if candidate_alias == alias_root
                && should_replace_opencode_match(best.as_ref().map(|(_, rows)| rows), &rows)
            {
                best = Some((path, rows));
            }
        }
        if let Some((path, rows)) = best {
            return (path, Some(alias_root), rows);
        }
    } else if alias_roots.len() > 1 {
        warnings.push(format!(
            "Multiple same-name OpenCode usage paths matched {}; refusing ambiguous fallback: {}.",
            project_path.display_path,
            alias_roots.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    (PathBuf::new(), None, Vec::new())
}

fn should_replace_opencode_match(
    current: Option<&Vec<OpenCodeSessionRow>>,
    candidate: &[OpenCodeSessionRow],
) -> bool {
    let Some(current) = current else {
        return true;
    };
    candidate.len() > current.len()
        || (candidate.len() == current.len()
            && max_opencode_updated_at(candidate) > max_opencode_updated_at(current))
}

fn max_opencode_updated_at(rows: &[OpenCodeSessionRow]) -> Option<i64> {
    rows.iter().filter_map(|row| row.time_updated).max()
}

fn read_codex_rollout_usage(path: &str, warnings: &mut Vec<String>) -> Option<TokenBreakdown> {
    if path.trim().is_empty() {
        return None;
    }
    let path = PathBuf::from(path);
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => return None,
    };
    let mut last = None;
    for line in BufReader::new(file)
        .lines()
        .map_while(std::result::Result::ok)
    {
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(usage) = value
            .pointer("/payload/info/total_token_usage")
            .and_then(parse_codex_usage_value)
        else {
            continue;
        };
        last = Some(usage);
    }
    if last.is_none() && warnings.len() < 6 {
        warnings.push(format!(
            "No total_token_usage found in Codex rollout {}.",
            path.display()
        ));
    }
    last
}

fn parse_codex_usage_value(value: &Value) -> Option<TokenBreakdown> {
    Some(TokenBreakdown {
        input: json_u64(value, "input_tokens"),
        output: json_u64(value, "output_tokens"),
        reasoning: json_u64(value, "reasoning_output_tokens"),
        cached_input: json_u64(value, "cached_input_tokens"),
        total: json_u64(value, "total_tokens"),
        cache_read: 0,
        cache_write: 0,
    })
}

fn codex_state_db_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(value) = env::var("CODEX_SQLITE_HOME") {
        candidates.push(PathBuf::from(value).join("state_5.sqlite"));
    }
    if let Ok(value) = env::var("CODEX_HOME") {
        let home = PathBuf::from(value);
        candidates.push(home.join("state_5.sqlite"));
        candidates.push(home.join("sqlite").join("state_5.sqlite"));
    }
    if let Some(home) = dirs::home_dir() {
        let codex_home = home.join(".codex");
        candidates.push(codex_home.join("state_5.sqlite"));
        candidates.push(codex_home.join("sqlite").join("state_5.sqlite"));
    }
    dedupe_paths(candidates)
}

fn opencode_db_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(value) = env::var("OPENCODE_DATA_HOME") {
        candidates.push(PathBuf::from(value).join("opencode.db"));
    }
    if let Ok(value) = env::var("XDG_DATA_HOME") {
        candidates.push(PathBuf::from(value).join("opencode").join("opencode.db"));
    }
    if let Some(data_dir) = dirs::data_dir() {
        candidates.push(data_dir.join("opencode").join("opencode.db"));
        candidates.push(data_dir.join("ai.opencode.desktop").join("opencode.db"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(
            home.join(".local")
                .join("share")
                .join("opencode")
                .join("opencode.db"),
        );
        candidates.push(
            home.join("Library")
                .join("Application Support")
                .join("ai.opencode.desktop")
                .join("opencode.db"),
        );
        candidates.push(
            home.join("AppData")
                .join("Roaming")
                .join("opencode")
                .join("opencode.db"),
        );
        candidates.push(
            home.join("AppData")
                .join("Local")
                .join("opencode")
                .join("opencode.db"),
        );
        candidates.push(
            home.join("AppData")
                .join("Roaming")
                .join("ai.opencode.desktop")
                .join("opencode.db"),
        );
    }
    dedupe_paths(candidates)
}

fn open_readonly(path: &Path) -> Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("Failed to open SQLite database {}", path.display()))
}

fn existing_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.into_iter().filter(|path| path.is_file()).collect()
}

fn empty_source(source: &str, warning: &str) -> AgentUsageSourceSummary {
    AgentUsageSourceSummary {
        source: source.to_string(),
        available: false,
        data_path: None,
        records: 0,
        non_cached_total_tokens: 0,
        total_tokens: 0,
        cost: None,
        tokens: TokenBreakdown::default(),
        latest_updated_at_ms: None,
        recent: Vec::new(),
        warnings: vec![warning.to_string()],
    }
}

fn error_source(
    source: &str,
    path: Option<PathBuf>,
    error: anyhow::Error,
) -> AgentUsageSourceSummary {
    let mut summary = empty_source(source, &format!("{error:#}"));
    summary.data_path = path.map(|path| path.display().to_string());
    summary
}

fn normalize_path_string(path: &Path) -> String {
    normalize_stored_path(&path.to_string_lossy())
}

fn normalize_stored_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    if trimmed.is_empty() {
        normalized
    } else {
        trimmed.to_string()
    }
}

fn child_path_pattern(path: &str) -> String {
    format!("{}/", normalize_stored_path(path))
}

fn lower_path_starts_with(value: &str, prefix: &str) -> bool {
    value
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
}

fn safe_title(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "Untitled".to_string()
    } else {
        trimmed.chars().take(120).collect()
    }
}

fn format_opencode_model(raw: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| value.get("model").and_then(Value::as_str))
                .map(str::to_string)
        })
        .unwrap_or_else(|| raw.chars().take(120).collect())
}

fn json_u64(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or_default()
}

fn signed_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn max_opt_i64(current: Option<i64>, candidate: Option<i64>) -> Option<i64> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in paths {
        if !out.iter().any(|existing| existing == &path) {
            out.push(path);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::{
        fs::{self, File},
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn aggregates_codex_threads_and_rollout_usage() {
        let dir = temp_dir("codex-usage");
        let db_path = dir.join("state_5.sqlite");
        let rollout_path = dir.join("rollout.jsonl");
        write_rollout(&rollout_path, 11, 7, 3, 5, 21);
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_codex_schema(&conn);
        conn.execute(
            "insert into threads (id, rollout_path, cwd, title, tokens_used, model, updated_at_ms)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "thread-1",
                rollout_path.display().to_string(),
                "/repo/app",
                "Usage work",
                21_i64,
                "gpt-test",
                123_i64
            ],
        )
        .expect("insert");
        drop(conn);

        let mut warnings = Vec::new();
        let usage = read_codex_usage_from_db("/repo/app", &db_path, &mut warnings).expect("usage");
        assert!(usage.available);
        assert_eq!(usage.records, 1);
        assert_eq!(usage.total_tokens, 21);
        assert_eq!(usage.non_cached_total_tokens, 14);
        assert_eq!(usage.tokens.input, 11);
        assert_eq!(usage.tokens.cached_input, 7);
        assert_eq!(usage.recent[0].non_cached_total_tokens, 14);
        assert_eq!(usage.recent[0].model.as_deref(), Some("gpt-test"));
    }

    #[test]
    fn scans_later_codex_databases_and_matches_child_paths() {
        let dir = temp_dir("codex-multi-db");
        let empty_db_path = dir.join("empty-state_5.sqlite");
        let matched_db_path = dir.join("matched-state_5.sqlite");
        let empty_conn = Connection::open(&empty_db_path).expect("open empty fixture db");
        create_codex_schema(&empty_conn);
        drop(empty_conn);

        let rollout_path = dir.join("rollout-child.jsonl");
        write_rollout(&rollout_path, 20, 0, 4, 1, 25);
        let matched_conn = Connection::open(&matched_db_path).expect("open matched fixture db");
        create_codex_schema(&matched_conn);
        matched_conn
            .execute(
                "insert into threads (id, rollout_path, cwd, title, tokens_used, model, updated_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    "thread-child",
                    rollout_path.display().to_string(),
                    "/repo/app/packages/web",
                    "Child package work",
                    25_i64,
                    "gpt-test",
                    321_i64
                ],
            )
            .expect("insert child");
        matched_conn
            .execute(
                "insert into threads (id, rollout_path, cwd, title, tokens_used, model, updated_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    "thread-sibling-prefix",
                    rollout_path.display().to_string(),
                    "/repo/application",
                    "Should not match",
                    999_i64,
                    "gpt-test",
                    322_i64
                ],
            )
            .expect("insert sibling prefix");
        drop(matched_conn);

        let mut warnings = Vec::new();
        let matcher = ProjectPathMatcher::new(Path::new("/repo/app"));
        let usage = read_codex_usage_from_dbs(
            &matcher,
            &[empty_db_path.clone(), matched_db_path.clone()],
            &mut warnings,
        )
        .expect("usage");

        assert!(usage.available);
        assert_eq!(usage.records, 1);
        assert_eq!(usage.total_tokens, 25);
        assert_eq!(usage.non_cached_total_tokens, 25);
        assert_eq!(
            usage.data_path.as_deref(),
            Some(matched_db_path.to_str().unwrap())
        );
    }

    #[test]
    fn codex_falls_back_to_unique_same_name_project_path() {
        let dir = temp_dir("codex-alias-path");
        let db_path = dir.join("state_5.sqlite");
        let rollout_path = dir.join("rollout-alias.jsonl");
        write_rollout(&rollout_path, 30, 5, 4, 1, 40);
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_codex_schema(&conn);
        conn.execute(
            "insert into threads (id, rollout_path, cwd, title, tokens_used, model, updated_at_ms)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "thread-alias",
                rollout_path.display().to_string(),
                "/repo/local/mind2realistic",
                "Moved project work",
                40_i64,
                "gpt-test",
                654_i64
            ],
        )
        .expect("insert alias");
        drop(conn);

        let mut warnings = Vec::new();
        let usage =
            read_codex_usage_from_db("/repo/archive/mind2realistic", &db_path, &mut warnings)
                .expect("usage");

        assert!(usage.available);
        assert_eq!(usage.records, 1);
        assert_eq!(usage.total_tokens, 40);
        assert!(
            usage
                .warnings
                .iter()
                .any(|warning| warning.contains("/repo/local/mind2realistic")),
            "expected alias warning, got {:?}",
            usage.warnings
        );
    }

    #[test]
    fn aggregates_opencode_sessions() {
        let dir = temp_dir("opencode-usage");
        let db_path = dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_opencode_schema(&conn);
        conn.execute(
            "insert into project (id, worktree) values ('p1', '/repo/app')",
            [],
        )
        .expect("project");
        conn.execute(
            "insert into session (
                id, project_id, directory, title, agent, model, cost, tokens_input,
                tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                time_updated, path
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                "ses-1",
                "p1",
                "/repo/app",
                "OpenCode work",
                "build",
                r#"{"id":"model-a"}"#,
                1.25_f64,
                10_i64,
                2_i64,
                3_i64,
                4_i64,
                5_i64,
                456_i64,
                "/repo/app",
            ],
        )
        .expect("session");
        drop(conn);

        let usage = read_opencode_usage_from_db("/repo/app", &db_path).expect("usage");
        assert!(usage.available);
        assert_eq!(usage.records, 1);
        assert_eq!(usage.total_tokens, 24);
        assert_eq!(usage.non_cached_total_tokens, 15);
        assert_eq!(usage.tokens.cache_read, 4);
        assert_eq!(usage.recent[0].non_cached_total_tokens, 15);
        assert_eq!(usage.recent[0].model.as_deref(), Some("model-a"));
    }

    #[test]
    fn opencode_matches_child_paths_without_matching_prefix_siblings() {
        let dir = temp_dir("opencode-child-path");
        let db_path = dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_opencode_schema(&conn);
        conn.execute(
            "insert into project (id, worktree) values ('p1', '/repo/app/packages/web')",
            [],
        )
        .expect("project child");
        conn.execute(
            "insert into project (id, worktree) values ('p2', '/repo/application')",
            [],
        )
        .expect("project sibling prefix");
        conn.execute(
            "insert into session (
                id, project_id, directory, title, agent, model, cost, tokens_input,
                tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                time_updated, path
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                "ses-child",
                "p1",
                "/repo/app/packages/web",
                "Child OpenCode work",
                "build",
                r#"{"id":"model-a"}"#,
                1.0_f64,
                10_i64,
                2_i64,
                3_i64,
                4_i64,
                5_i64,
                456_i64,
                "/repo/app/packages/web",
            ],
        )
        .expect("child session");
        conn.execute(
            "insert into session (
                id, project_id, directory, title, agent, model, cost, tokens_input,
                tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                time_updated, path
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                "ses-sibling-prefix",
                "p2",
                "/repo/application",
                "Should not match",
                "build",
                r#"{"id":"model-b"}"#,
                1.0_f64,
                1000_i64,
                200_i64,
                300_i64,
                400_i64,
                500_i64,
                457_i64,
                "/repo/application",
            ],
        )
        .expect("sibling prefix session");
        drop(conn);

        let usage = read_opencode_usage_from_db("/repo/app", &db_path).expect("usage");
        assert!(usage.available);
        assert_eq!(usage.records, 1);
        assert_eq!(usage.total_tokens, 24);
        assert_eq!(usage.recent[0].id, "ses-child");
    }

    #[test]
    fn opencode_falls_back_to_unique_same_name_project_path() {
        let dir = temp_dir("opencode-alias-path");
        let db_path = dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_opencode_schema(&conn);
        conn.execute(
            "insert into project (id, worktree) values ('p1', '/repo/local/mind2realistic')",
            [],
        )
        .expect("project alias");
        conn.execute(
            "insert into session (
                id, project_id, directory, title, agent, model, cost, tokens_input,
                tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                time_updated, path
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                "ses-alias",
                "p1",
                "/repo/local/mind2realistic",
                "Moved OpenCode work",
                "build",
                r#"{"id":"model-a"}"#,
                1.0_f64,
                10_i64,
                2_i64,
                3_i64,
                4_i64,
                5_i64,
                654_i64,
                "/repo/local/mind2realistic",
            ],
        )
        .expect("alias session");
        drop(conn);

        let usage =
            read_opencode_usage_from_db("/repo/archive/mind2realistic", &db_path).expect("usage");

        assert!(usage.available);
        assert_eq!(usage.records, 1);
        assert_eq!(usage.total_tokens, 24);
        assert_eq!(usage.recent[0].id, "ses-alias");
        assert!(
            usage
                .warnings
                .iter()
                .any(|warning| warning.contains("/repo/local/mind2realistic")),
            "expected alias warning, got {:?}",
            usage.warnings
        );
    }

    fn create_codex_schema(conn: &Connection) {
        conn.execute_batch(
            "create table threads (
                id text primary key,
                rollout_path text not null,
                created_at integer not null default 0,
                updated_at integer not null default 0,
                source text not null default '',
                model_provider text not null default '',
                cwd text not null,
                title text not null,
                sandbox_policy text not null default '',
                approval_mode text not null default '',
                tokens_used integer not null default 0,
                model text,
                updated_at_ms integer
            );",
        )
        .expect("codex schema");
    }

    fn create_opencode_schema(conn: &Connection) {
        conn.execute_batch(
            "create table project (
                id text primary key,
                worktree text not null
            );
            create table session (
                id text primary key,
                project_id text not null,
                directory text not null,
                title text not null,
                agent text,
                model text,
                cost real default 0 not null,
                tokens_input integer default 0 not null,
                tokens_output integer default 0 not null,
                tokens_reasoning integer default 0 not null,
                tokens_cache_read integer default 0 not null,
                tokens_cache_write integer default 0 not null,
                time_updated integer,
                path text
            );",
        )
        .expect("opencode schema");
    }

    fn write_rollout(
        path: &Path,
        input: u64,
        cached: u64,
        output: u64,
        reasoning: u64,
        total: u64,
    ) {
        let mut file = File::create(path).expect("rollout");
        writeln!(
            file,
            "{{\"payload\":{{\"info\":{{\"total_token_usage\":{{\"input_tokens\":{},\"cached_input_tokens\":{},\"output_tokens\":{},\"reasoning_output_tokens\":{},\"total_tokens\":{}}}}}}}}}",
            input, cached, output, reasoning, total
        )
        .expect("write");
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = env::temp_dir().join(format!("vibehub-{prefix}-{nanos}"));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }
}
