use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OpenFlags};
use serde::Serialize;
use serde_json::Value;
use std::{
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
    pub total_tokens: u64,
    pub cost: Option<f64>,
    pub updated_at_ms: Option<i64>,
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

pub fn read_local_agent_usage(project_path: impl AsRef<Path>) -> Result<LocalAgentUsageOverview> {
    let project_path = normalize_path_string(project_path.as_ref());
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
        total_tokens: codex.total_tokens.saturating_add(opencode.total_tokens),
        source_count: [codex.available, opencode.available]
            .into_iter()
            .filter(|available| *available)
            .count(),
        project_path,
        generated_at: Utc::now().to_rfc3339(),
        codex,
        opencode,
        warnings,
    })
}

fn read_codex_usage(project_path: &str) -> AgentUsageSourceSummary {
    let mut warnings = Vec::new();
    let Some(db_path) = first_existing_path(codex_state_db_candidates()) else {
        return empty_source(
            "codex",
            "Codex state database was not found in CODEX_SQLITE_HOME, CODEX_HOME, or ~/.codex.",
        );
    };

    let result = read_codex_usage_from_db(project_path, &db_path, &mut warnings);
    match result {
        Ok(mut summary) => {
            summary.data_path = Some(db_path.display().to_string());
            summary.warnings = warnings;
            summary
        }
        Err(error) => error_source("codex", Some(db_path), error),
    }
}

fn read_codex_usage_from_db(
    project_path: &str,
    db_path: &Path,
    warnings: &mut Vec<String>,
) -> Result<AgentUsageSourceSummary> {
    let conn = open_readonly(db_path)?;
    let mut stmt = conn.prepare(
        "select id, rollout_path, title, model_provider, model, tokens_used, updated_at_ms \
         from threads where cwd = ?1 order by updated_at_ms desc, updated_at desc, id desc",
    )?;
    let rows = stmt
        .query_map(params![project_path], |row| {
            Ok(CodexThreadRow {
                id: row.get(0)?,
                rollout_path: row.get(1)?,
                title: row.get(2)?,
                model_provider: row.get(3).ok(),
                model: row.get(4).ok(),
                tokens_used: signed_to_u64(row.get::<_, i64>(5).unwrap_or_default()),
                updated_at_ms: row.get(6).ok(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut tokens = TokenBreakdown::default();
    let mut recent = Vec::new();
    let mut latest_updated_at_ms = None;

    for row in rows.iter() {
        tokens.total = tokens.total.saturating_add(row.tokens_used);
        latest_updated_at_ms = max_opt_i64(latest_updated_at_ms, row.updated_at_ms);
        let usage = read_codex_rollout_usage(&row.rollout_path, warnings);
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
                total_tokens: row.tokens_used,
                cost: None,
                updated_at_ms: row.updated_at_ms,
            });
        }
    }

    Ok(AgentUsageSourceSummary {
        source: "codex".to_string(),
        available: !rows.is_empty(),
        data_path: None,
        records: rows.len(),
        total_tokens: tokens.total,
        cost: None,
        tokens,
        latest_updated_at_ms,
        recent,
        warnings: Vec::new(),
    })
}

fn read_opencode_usage(project_path: &str) -> AgentUsageSourceSummary {
    let mut warnings = Vec::new();
    let Some(db_path) = first_existing_path(opencode_db_candidates()) else {
        return empty_source(
            "opencode",
            "OpenCode database was not found in XDG data, ~/.local/share/opencode, Application Support, or AppData candidates.",
        );
    };

    let result = read_opencode_usage_from_db(project_path, &db_path);
    match result {
        Ok(mut summary) => {
            summary.data_path = Some(db_path.display().to_string());
            summary.warnings.append(&mut warnings);
            summary
        }
        Err(error) => error_source("opencode", Some(db_path), error),
    }
}

fn read_opencode_usage_from_db(
    project_path: &str,
    db_path: &Path,
) -> Result<AgentUsageSourceSummary> {
    let conn = open_readonly(db_path)?;
    let mut stmt = conn.prepare(
        "select s.id, s.title, s.agent, s.model, s.cost, s.tokens_input, s.tokens_output, \
                s.tokens_reasoning, s.tokens_cache_read, s.tokens_cache_write, s.time_updated \
         from session s \
         left join project p on p.id = s.project_id \
         where p.worktree = ?1 or s.directory = ?1 or s.path = ?1 \
         order by s.time_updated desc, s.id desc",
    )?;
    let rows = stmt
        .query_map(params![project_path], |row| {
            Ok(OpenCodeSessionRow {
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
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

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
                total_tokens: total,
                cost: Some(row.cost),
                updated_at_ms: row.time_updated,
            });
        }
    }

    Ok(AgentUsageSourceSummary {
        source: "opencode".to_string(),
        available: !rows.is_empty(),
        data_path: None,
        records: rows.len(),
        total_tokens: tokens.total,
        cost: Some(cost),
        tokens,
        latest_updated_at_ms,
        recent,
        warnings: Vec::new(),
    })
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

fn first_existing_path(paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths.into_iter().find(|path| path.is_file())
}

fn empty_source(source: &str, warning: &str) -> AgentUsageSourceSummary {
    AgentUsageSourceSummary {
        source: source.to_string(),
        available: false,
        data_path: None,
        records: 0,
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
    path.to_string_lossy().replace('\\', "/")
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
        .expect("schema");
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
        assert_eq!(usage.tokens.input, 11);
        assert_eq!(usage.tokens.cached_input, 7);
        assert_eq!(usage.recent[0].model.as_deref(), Some("gpt-test"));
    }

    #[test]
    fn aggregates_opencode_sessions() {
        let dir = temp_dir("opencode-usage");
        let db_path = dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open fixture db");
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
        .expect("schema");
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
        assert_eq!(usage.tokens.cache_read, 4);
        assert_eq!(usage.recent[0].model.as_deref(), Some("model-a"));
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
