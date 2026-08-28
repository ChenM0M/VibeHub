use super::domain::{V3Error, V3ErrorCategory, V3EventEnvelope};
use super::lifecycle::TaskLifecycleProjection;
use super::orchestration::OrchestrationProjection;
use super::project_memory::MemoryProjection;
use super::projection::{self, SessionProjection, V3Projection};
use super::routing::SessionTaskBinding;
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) const STORE_FORMAT: &str = "2";
pub(crate) const STORE_MODEL_VERSION: &str = "v3-sqlite-wal-2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MigrationFault {
    DiskFull,
    RenameFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SourceSnapshot {
    present: bool,
    bytes: u64,
    sha256: String,
    backup_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MigrationManifest {
    schema_version: String,
    migration_version: String,
    project_id: String,
    source_store_format: String,
    target_store_format: String,
    target_model_version: String,
    created_at: String,
    events: SourceSnapshot,
    projection: SourceSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexedStoreStatus {
    pub database_path: PathBuf,
    pub event_count: u64,
    pub source_byte_offset: u64,
    pub source_file_bytes: u64,
    pub last_global_seq: u64,
    pub last_event_timestamp: Option<String>,
    pub state: String,
}

#[derive(Debug)]
pub(crate) struct IndexedProjectStore {
    path: PathBuf,
}

impl IndexedProjectStore {
    pub(crate) fn quarantine_corrupt(path: &Path) -> Result<PathBuf, V3Error> {
        quarantine_failed_index(path)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn migrate_from_jsonl(
        index_path: &Path,
        events_path: &Path,
        projection_path: &Path,
        backups_root: &Path,
        migration_marker: &Path,
        project_id: &str,
        fault: Option<MigrationFault>,
    ) -> Result<Self, V3Error> {
        validate_source_path(events_path, "events.jsonl")?;
        validate_source_path(projection_path, "projection.json")?;
        validate_migration_target(index_path)?;
        validate_directory_target(backups_root)?;

        let events = source_snapshot(events_path, "events.jsonl")?;
        let projection = source_snapshot(projection_path, "projection.json")?;
        let manifest = MigrationManifest {
            schema_version: "1.0".to_owned(),
            migration_version: "v3-store-format-1-to-2.1".to_owned(),
            project_id: project_id.to_owned(),
            source_store_format: "1".to_owned(),
            target_store_format: STORE_FORMAT.to_owned(),
            target_model_version: STORE_MODEL_VERSION.to_owned(),
            created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            events,
            projection,
        };
        ensure_backup(backups_root, events_path, projection_path, &manifest)?;
        if fault == Some(MigrationFault::DiskFull) {
            return Err(migration_error(
                "V3_INDEX_MIGRATION_DISK_FULL",
                index_path,
                "injected disk-full failure before derived index creation",
            ));
        }

        let temp_path = index_path.with_extension("sqlite3.migrating");
        validate_temp_index(&temp_path)?;
        let temp_store = match Self::open(&temp_path) {
            Ok(store) => store,
            Err(error) if temp_path.exists() => {
                quarantine_failed_temp(&temp_path)?;
                Self::open(&temp_path).map_err(|retry| {
                    migration_error(
                        "V3_INDEX_MIGRATION_BUILD_FAILED",
                        index_path,
                        format!("{}; retry: {}", error.message, retry.message),
                    )
                })?
            }
            Err(error) => return Err(error),
        };
        temp_store
            .reset_and_sync(events_path, project_id)
            .map_err(|error| {
                migration_error("V3_INDEX_MIGRATION_BUILD_FAILED", index_path, error.message)
            })?;
        let indexed_events = temp_store.all_events()?;
        let replayed = projection::fold(project_id, &indexed_events);
        let materialized = temp_store.full_projection(project_id)?;
        if materialized != replayed {
            return Err(migration_error(
                "V3_INDEX_MIGRATION_VERIFY_FAILED",
                index_path,
                "materialized shards differ from a complete JSONL replay",
            )
            .with_detail(
                "mismatched_fields",
                projection_mismatched_fields(&materialized, &replayed),
            ));
        }
        let events_after = source_snapshot(events_path, "events.jsonl")?;
        let projection_after = source_snapshot(projection_path, "projection.json")?;
        if events_after != manifest.events || projection_after != manifest.projection {
            return Err(migration_error(
                "V3_INDEX_MIGRATION_SOURCE_CHANGED",
                index_path,
                "source files changed while the derived index was being built",
            ));
        }
        temp_store.checkpoint()?;
        drop(temp_store);
        cleanup_sqlite_sidecars(&temp_path)?;
        if fault == Some(MigrationFault::RenameFailed) {
            return Err(migration_error(
                "V3_INDEX_MIGRATION_RENAME_FAILED",
                index_path,
                "injected atomic rename failure",
            ));
        }
        fs::rename(&temp_path, index_path).map_err(|error| {
            migration_error(
                "V3_INDEX_MIGRATION_RENAME_FAILED",
                index_path,
                error.to_string(),
            )
        })?;
        if let Err(error) = write_migration_marker(migration_marker, &manifest, index_path) {
            let _ = fs::rename(index_path, &temp_path);
            return Err(error);
        }
        sync_parent(index_path)?;
        Self::open(index_path)
    }

    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, V3Error> {
        let path = path.as_ref().to_owned();
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(V3Error::new(
                    "V3_INDEX_PATH_SYMLINK",
                    V3ErrorCategory::PermissionDenied,
                    false,
                    "derived index path must not be a symbolic link",
                )
                .with_detail("path", path.to_string_lossy().to_string()));
            }
            Ok(metadata) if !metadata.is_file() => {
                return Err(V3Error::new(
                    "V3_INDEX_PATH_INVALID",
                    V3ErrorCategory::Validation,
                    false,
                    "derived index path must be a regular file",
                )
                .with_detail("path", path.to_string_lossy().to_string()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("V3_INDEX_METADATA_FAILED")(error)),
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error("V3_INDEX_CREATE_FAILED"))?;
        }
        let store = Self { path };
        let connection = store.connection()?;
        let started = Instant::now();
        loop {
            let result = match store.schema_ready(&connection) {
                Ok(true) => break,
                Ok(false) => store.initialize(&connection),
                Err(error) => Err(error),
            };
            match result {
                Ok(()) => break,
                Err(error) if sqlite_busy(&error) && started.elapsed() < Duration::from_secs(3) => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error),
            }
        }
        Ok(store)
    }

    pub(crate) fn sync_from_jsonl(
        &self,
        events_path: &Path,
        project_id: &str,
    ) -> Result<IndexedStoreStatus, V3Error> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite_error("V3_INDEX_TRANSACTION_FAILED"))?;
        // Sample the source length only after the SQLite transaction snapshot
        // is established. Otherwise a concurrent writer can commit a newer
        // watermark while this connection waits, making an old file-length
        // sample look like index corruption.
        let source_file_bytes = source_file_bytes(events_path)?;
        let source_byte_offset = meta_u64(&transaction, "source_byte_offset")?.unwrap_or(0);
        if source_byte_offset > source_file_bytes {
            return Err(V3Error::new(
                "V3_INDEX_AHEAD_OF_SOURCE",
                V3ErrorCategory::CorruptLog,
                false,
                "indexed source watermark is ahead of events.jsonl",
            )
            .with_detail("source_byte_offset", source_byte_offset)
            .with_detail("source_file_bytes", source_file_bytes)
            .with_detail("index_path", self.path.to_string_lossy().to_string()));
        }

        let mut touched_tasks = BTreeSet::new();
        let mut touched_sessions = BTreeSet::new();
        let mut memory_touched = false;
        let mut current_offset = source_byte_offset;
        let mut next_sequence = max_global_sequence(&transaction)?.saturating_add(1);
        if current_offset < source_file_bytes {
            let file = File::open(events_path).map_err(io_error("V3_EVENT_READ_FAILED"))?;
            let mut reader = BufReader::new(file);
            reader
                .seek(SeekFrom::Start(current_offset))
                .map_err(io_error("V3_EVENT_SEEK_FAILED"))?;
            while current_offset < source_file_bytes {
                let line_offset = current_offset;
                let mut line = Vec::new();
                let read = reader
                    .read_until(b'\n', &mut line)
                    .map_err(io_error("V3_EVENT_READ_FAILED"))?;
                if read == 0 {
                    break;
                }
                if !line.ends_with(b"\n") {
                    // A different process may currently hold the JSONL lock
                    // and be writing the next record. Commit only complete
                    // lines; a lock-owning read or the writer's own follow-up
                    // sync will catch up the remaining tail. Genuine crashed
                    // tails are quarantined by event_store while holding the
                    // sequencing lock before this method is called.
                    break;
                }
                current_offset = current_offset.saturating_add(read as u64);
                line.pop();
                if line.ends_with(b"\r") {
                    line.pop();
                }
                if line.is_empty() {
                    return Err(V3Error::new(
                        "V3_CORRUPT_LOG",
                        V3ErrorCategory::CorruptLog,
                        false,
                        "events.jsonl contains an empty record",
                    )
                    .with_detail("source_offset", line_offset));
                }
                let event: V3EventEnvelope = serde_json::from_slice(&line).map_err(|error| {
                    V3Error::new(
                        "V3_CORRUPT_LOG",
                        V3ErrorCategory::CorruptLog,
                        false,
                        error.to_string(),
                    )
                    .with_detail("source_offset", line_offset)
                })?;
                if event.project_id.0 != project_id {
                    return Err(V3Error::new(
                        "V3_INDEX_PROJECT_MISMATCH",
                        V3ErrorCategory::ScopeMismatch,
                        false,
                        "event project identity does not match the indexed project",
                    )
                    .with_detail("expected_project_id", project_id.to_owned())
                    .with_detail("event_project_id", event.project_id.0));
                }
                if event.task_id.0.trim().is_empty()
                    || event.task_id.0.len() > 240
                    || event.task_id.0.contains(['/', '\\'])
                    || matches!(event.task_id.0.as_str(), "." | "..")
                {
                    return Err(V3Error::new(
                        "V3_INDEX_TASK_ID_INVALID",
                        V3ErrorCategory::Validation,
                        false,
                        "event task_id is not a bounded path-safe identity",
                    )
                    .with_detail("task_id", event.task_id.0));
                }
                insert_event(
                    &transaction,
                    next_sequence,
                    line_offset,
                    read as u64,
                    &event,
                )?;
                touched_tasks.insert(event.task_id.0.clone());
                if let Some(session_id) = &event.session_id {
                    touched_sessions.insert(session_id.0.clone());
                }
                memory_touched |= event.event_type.starts_with("memory.");
                next_sequence = next_sequence.saturating_add(1);
            }
        }

        for task_id in touched_tasks {
            refresh_task_shards(&transaction, project_id, &task_id)?;
        }
        for session_id in touched_sessions {
            refresh_session_shards(&transaction, project_id, &session_id)?;
        }
        let memory_shard_exists = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM project_memory WHERE project_id = ?1)",
                params![project_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
        if memory_touched || !memory_shard_exists {
            refresh_memory_shard(&transaction, project_id)?;
        }
        set_meta(&transaction, "store_format", STORE_FORMAT)?;
        set_meta(&transaction, "model_version", STORE_MODEL_VERSION)?;
        set_meta(
            &transaction,
            "source_byte_offset",
            &current_offset.to_string(),
        )?;
        transaction
            .commit()
            .map_err(sqlite_error("V3_INDEX_COMMIT_FAILED"))?;
        self.status(events_path)
    }

    pub(crate) fn reset_and_sync(
        &self,
        events_path: &Path,
        project_id: &str,
    ) -> Result<IndexedStoreStatus, V3Error> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite_error("V3_INDEX_TRANSACTION_FAILED"))?;
        transaction
            .execute_batch(
                "DELETE FROM task_projections;
                 DELETE FROM session_projections;
                 DELETE FROM session_bindings;
                 DELETE FROM worktree_projections;
                 DELETE FROM project_memory;
                 DELETE FROM events;
                 DELETE FROM meta WHERE key NOT IN ('store_format', 'model_version');",
            )
            .map_err(sqlite_error("V3_INDEX_RESET_FAILED"))?;
        set_meta(&transaction, "source_byte_offset", "0")?;
        transaction
            .commit()
            .map_err(sqlite_error("V3_INDEX_COMMIT_FAILED"))?;
        self.sync_from_jsonl(events_path, project_id)
    }

    pub(crate) fn status(&self, events_path: &Path) -> Result<IndexedStoreStatus, V3Error> {
        let connection = self.connection()?;
        let source_byte_offset = meta_u64(&connection, "source_byte_offset")?.unwrap_or(0);
        let event_count = connection
            .query_row("SELECT COUNT(*) FROM events", [], |row| {
                row.get::<_, u64>(0)
            })
            .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
        let last_global_seq = max_global_sequence(&connection)?;
        let last_event_timestamp = connection
            .query_row(
                "SELECT recorded_at FROM events ORDER BY global_seq DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
        let source_file_bytes = source_file_bytes(events_path)?;
        let state = if source_byte_offset == source_file_bytes {
            "synced"
        } else if source_byte_offset < source_file_bytes {
            "behind"
        } else {
            "ahead"
        };
        Ok(IndexedStoreStatus {
            database_path: self.path.clone(),
            event_count,
            source_byte_offset,
            source_file_bytes,
            last_global_seq,
            last_event_timestamp,
            state: state.to_owned(),
        })
    }

    pub(crate) fn event_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<V3EventEnvelope>, V3Error> {
        self.query_optional_event(
            "SELECT event_json FROM events WHERE idempotency_key = ?1",
            key,
        )
    }

    pub(crate) fn aggregate_version(&self, aggregate_id: &str) -> Result<u64, V3Error> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT COALESCE(MAX(aggregate_version), 0) FROM events WHERE aggregate_id = ?1",
                params![aggregate_id],
                |row| row.get(0),
            )
            .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))
    }

    pub(crate) fn all_events(&self) -> Result<Vec<V3EventEnvelope>, V3Error> {
        self.query_events(
            "SELECT event_json FROM events ORDER BY global_seq",
            params![],
        )
    }

    pub(crate) fn task_events(&self, task_id: &str) -> Result<Vec<V3EventEnvelope>, V3Error> {
        self.query_events(
            "SELECT event_json FROM events WHERE task_id = ?1 ORDER BY global_seq",
            params![task_id],
        )
    }

    pub(crate) fn aggregate_events(
        &self,
        aggregate_id: &str,
    ) -> Result<Vec<V3EventEnvelope>, V3Error> {
        self.query_events(
            "SELECT event_json FROM events WHERE aggregate_id = ?1 ORDER BY global_seq",
            params![aggregate_id],
        )
    }

    pub(crate) fn task_projection(
        &self,
        task_id: &str,
    ) -> Result<Option<TaskLifecycleProjection>, V3Error> {
        self.query_optional_json(
            "SELECT projection_json FROM task_projections WHERE task_id = ?1",
            task_id,
        )
    }

    pub(crate) fn project_memory(
        &self,
        project_id: &str,
    ) -> Result<Option<MemoryProjection>, V3Error> {
        self.query_optional_json(
            "SELECT projection_json FROM project_memory WHERE project_id = ?1",
            project_id,
        )
    }

    pub(crate) fn orchestration_projection(
        &self,
        task_id: &str,
    ) -> Result<OrchestrationProjection, V3Error> {
        let events = self.task_events(task_id)?;
        Ok(super::orchestration::fold_task(task_id, &events))
    }

    pub(crate) fn full_projection(&self, project_id: &str) -> Result<V3Projection, V3Error> {
        let connection = self.connection()?;
        let source_event_ids = query_strings(
            &connection,
            "SELECT event_id FROM events ORDER BY global_seq",
        )?;
        let mut aggregate_versions = BTreeMap::new();
        {
            let mut statement = connection
                .prepare(
                    "SELECT aggregate_id, MAX(aggregate_version) FROM events GROUP BY aggregate_id",
                )
                .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
                })
                .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
            for row in rows {
                let (aggregate_id, version) = row.map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
                aggregate_versions.insert(aggregate_id, version);
            }
        }
        let tasks = query_json_map::<TaskLifecycleProjection>(
            &connection,
            "SELECT task_id, projection_json FROM task_projections ORDER BY task_id",
        )?;
        let sessions = query_json_map::<SessionProjection>(
            &connection,
            "SELECT session_id, projection_json FROM session_projections ORDER BY session_id",
        )?;
        let session_bindings = query_json_map::<SessionTaskBinding>(
            &connection,
            "SELECT session_id, projection_json FROM session_bindings ORDER BY session_id",
        )?;
        let worktrees = query_json_map(
            &connection,
            "SELECT worktree_id, projection_json FROM worktree_projections ORDER BY worktree_id",
        )?;
        let project_memory = self.project_memory(project_id)?;
        let mut unknown_event_types = Vec::new();
        {
            let mut statement = connection
                .prepare("SELECT event_type FROM events ORDER BY global_seq")
                .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
            for row in rows {
                let event_type = row.map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
                if !projection::is_known_event_type(&event_type) {
                    unknown_event_types.push(event_type);
                }
            }
        }
        let total_event_count = source_event_ids.len() as u64;
        let last_event_timestamp = connection
            .query_row(
                "SELECT recorded_at FROM events ORDER BY global_seq DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
        Ok(V3Projection {
            schema_version: "1.0".to_owned(),
            model_version: "v3-core-2".to_owned(),
            project_id: project_id.to_owned(),
            source_event_ids,
            aggregate_versions,
            tasks,
            sessions,
            session_bindings,
            worktrees,
            project_memory,
            unknown_event_types,
            total_event_count,
            last_event_timestamp,
        })
    }

    fn query_optional_event(
        &self,
        sql: &str,
        value: &str,
    ) -> Result<Option<V3EventEnvelope>, V3Error> {
        self.query_optional_json(sql, value)
    }

    fn query_optional_json<T: DeserializeOwned>(
        &self,
        sql: &str,
        value: &str,
    ) -> Result<Option<T>, V3Error> {
        let connection = self.connection()?;
        let json = connection
            .query_row(sql, params![value], |row| row.get::<_, String>(0))
            .optional()
            .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
        json.map(|json| decode_json(&json)).transpose()
    }

    fn query_events<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<Vec<V3EventEnvelope>, V3Error> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(sql)
            .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
        let rows = statement
            .query_map(params, |row| row.get::<_, String>(0))
            .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
        let mut events = Vec::new();
        for row in rows {
            events.push(decode_json(
                &row.map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?,
            )?);
        }
        Ok(events)
    }

    fn connection(&self) -> Result<Connection, V3Error> {
        let connection =
            Connection::open(&self.path).map_err(sqlite_error("V3_INDEX_OPEN_FAILED"))?;
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(sqlite_error("V3_INDEX_CONFIG_FAILED"))?;
        connection
            .execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")
            .map_err(sqlite_error("V3_INDEX_CONFIG_FAILED"))?;
        Ok(connection)
    }

    fn schema_ready(&self, connection: &Connection) -> Result<bool, V3Error> {
        let meta_exists = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='meta')",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(sqlite_error("V3_INDEX_SCHEMA_FAILED"))?;
        if !meta_exists {
            return Ok(false);
        }
        let format = connection
            .query_row(
                "SELECT value FROM meta WHERE key = 'store_format'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sqlite_error("V3_INDEX_SCHEMA_FAILED"))?;
        match format {
            None => Ok(false),
            Some(format) if format == STORE_FORMAT => {
                let model_version = connection
                    .query_row(
                        "SELECT value FROM meta WHERE key = 'model_version'",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(sqlite_error("V3_INDEX_SCHEMA_FAILED"))?;
                if model_version.as_deref() != Some(STORE_MODEL_VERSION) {
                    return Err(V3Error::new(
                        "V3_INDEX_MODEL_OUTDATED",
                        V3ErrorCategory::StaleResource,
                        true,
                        "derived event index uses an outdated projection model",
                    )
                    .with_detail("expected_model_version", STORE_MODEL_VERSION)
                    .with_detail("observed_model_version", model_version));
                }
                let table_count = connection
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master
                         WHERE type='table' AND name IN (
                            'events', 'task_projections', 'session_projections',
                            'session_bindings', 'worktree_projections', 'project_memory'
                         )",
                        [],
                        |row| row.get::<_, u64>(0),
                    )
                    .map_err(sqlite_error("V3_INDEX_SCHEMA_FAILED"))?;
                if table_count == 6 {
                    Ok(true)
                } else {
                    Err(V3Error::new(
                        "V3_INDEX_SCHEMA_INCOMPLETE",
                        V3ErrorCategory::CorruptLog,
                        false,
                        "derived event index is missing required format-2 tables",
                    )
                    .with_detail("expected_table_count", 6_u64)
                    .with_detail("observed_table_count", table_count))
                }
            }
            Some(format) => Err(V3Error::new(
                "V3_INDEX_FORMAT_UNSUPPORTED",
                V3ErrorCategory::Validation,
                false,
                "derived event index uses an unsupported store format",
            )
            .with_detail("expected_store_format", STORE_FORMAT)
            .with_detail("observed_store_format", format)),
        }
    }

    fn initialize(&self, connection: &Connection) -> Result<(), V3Error> {
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA synchronous=FULL;
                 PRAGMA foreign_keys=ON;
                 CREATE TABLE IF NOT EXISTS meta (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS events (
                    global_seq INTEGER PRIMARY KEY,
                    event_id TEXT NOT NULL UNIQUE,
                    source_offset INTEGER NOT NULL,
                    source_len INTEGER NOT NULL,
                    task_id TEXT NOT NULL,
                    aggregate_id TEXT NOT NULL,
                    aggregate_version INTEGER NOT NULL,
                    idempotency_key TEXT NOT NULL UNIQUE,
                    event_type TEXT NOT NULL,
                    session_id TEXT,
                    recorded_at TEXT NOT NULL,
                    event_json TEXT NOT NULL,
                    UNIQUE(aggregate_id, aggregate_version)
                 );
                 CREATE INDEX IF NOT EXISTS idx_events_task_seq
                    ON events(task_id, global_seq);
                 CREATE INDEX IF NOT EXISTS idx_events_aggregate_version
                    ON events(aggregate_id, aggregate_version);
                 CREATE UNIQUE INDEX IF NOT EXISTS idx_events_idempotency
                    ON events(idempotency_key);
                 CREATE UNIQUE INDEX IF NOT EXISTS idx_events_event_id
                    ON events(event_id);
                 CREATE INDEX IF NOT EXISTS idx_events_session_seq
                    ON events(session_id, global_seq);
                 CREATE TABLE IF NOT EXISTS task_projections (
                    task_id TEXT PRIMARY KEY,
                    projection_json TEXT NOT NULL,
                    last_global_seq INTEGER NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS session_projections (
                    session_id TEXT PRIMARY KEY,
                    task_id TEXT NOT NULL,
                    projection_json TEXT NOT NULL,
                    last_global_seq INTEGER NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_session_projection_task
                    ON session_projections(task_id);
                 CREATE TABLE IF NOT EXISTS session_bindings (
                    session_id TEXT PRIMARY KEY,
                    task_id TEXT NOT NULL,
                    projection_json TEXT NOT NULL,
                    last_global_seq INTEGER NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_session_binding_task
                    ON session_bindings(task_id);
                 CREATE TABLE IF NOT EXISTS worktree_projections (
                    worktree_id TEXT PRIMARY KEY,
                    task_id TEXT NOT NULL,
                    projection_json TEXT NOT NULL,
                    last_global_seq INTEGER NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_worktree_projection_task
                    ON worktree_projections(task_id);
                 CREATE TABLE IF NOT EXISTS project_memory (
                    project_id TEXT PRIMARY KEY,
                    projection_json TEXT NOT NULL,
                    last_global_seq INTEGER NOT NULL
                 );",
            )
            .map_err(sqlite_error("V3_INDEX_SCHEMA_FAILED"))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO meta(key, value) VALUES('store_format', ?1)",
                params![STORE_FORMAT],
            )
            .map_err(sqlite_error("V3_INDEX_SCHEMA_FAILED"))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO meta(key, value) VALUES('model_version', ?1)",
                params![STORE_MODEL_VERSION],
            )
            .map_err(sqlite_error("V3_INDEX_SCHEMA_FAILED"))?;
        let format: String = connection
            .query_row(
                "SELECT value FROM meta WHERE key = 'store_format'",
                [],
                |row| row.get(0),
            )
            .map_err(sqlite_error("V3_INDEX_SCHEMA_FAILED"))?;
        if format != STORE_FORMAT {
            return Err(V3Error::new(
                "V3_INDEX_FORMAT_UNSUPPORTED",
                V3ErrorCategory::Validation,
                false,
                "derived event index uses an unsupported store format",
            )
            .with_detail("expected_store_format", STORE_FORMAT)
            .with_detail("observed_store_format", format));
        }
        Ok(())
    }

    fn checkpoint(&self) -> Result<(), V3Error> {
        let connection = self.connection()?;
        let busy = connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                row.get::<_, u64>(0)
            })
            .map_err(sqlite_error("V3_INDEX_CHECKPOINT_FAILED"))?;
        if busy != 0 {
            return Err(V3Error::new(
                "V3_INDEX_CHECKPOINT_BUSY",
                V3ErrorCategory::StaleResource,
                true,
                "cannot finalize migration while SQLite readers are active",
            ));
        }
        Ok(())
    }
}

fn projection_mismatched_fields(
    actual: &V3Projection,
    expected: &V3Projection,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if actual.schema_version != expected.schema_version {
        fields.push("schema_version");
    }
    if actual.model_version != expected.model_version {
        fields.push("model_version");
    }
    if actual.project_id != expected.project_id {
        fields.push("project_id");
    }
    if actual.source_event_ids != expected.source_event_ids {
        fields.push("source_event_ids");
    }
    if actual.aggregate_versions != expected.aggregate_versions {
        fields.push("aggregate_versions");
    }
    if actual.tasks != expected.tasks {
        fields.push("tasks");
    }
    if actual.sessions != expected.sessions {
        fields.push("sessions");
    }
    if actual.session_bindings != expected.session_bindings {
        fields.push("session_bindings");
    }
    if actual.worktrees != expected.worktrees {
        fields.push("worktrees");
    }
    if actual.project_memory != expected.project_memory {
        fields.push("project_memory");
    }
    if actual.unknown_event_types != expected.unknown_event_types {
        fields.push("unknown_event_types");
    }
    if actual.total_event_count != expected.total_event_count {
        fields.push("total_event_count");
    }
    if actual.last_event_timestamp != expected.last_event_timestamp {
        fields.push("last_event_timestamp");
    }
    fields
}

fn insert_event(
    transaction: &Transaction<'_>,
    global_seq: u64,
    source_offset: u64,
    source_len: u64,
    event: &V3EventEnvelope,
) -> Result<(), V3Error> {
    let event_json =
        serde_json::to_string(event).map_err(json_error("V3_INDEX_SERIALIZE_FAILED"))?;
    transaction
        .execute(
            "INSERT INTO events(
                global_seq, event_id, source_offset, source_len, task_id,
                aggregate_id, aggregate_version, idempotency_key, event_type,
                session_id, recorded_at, event_json
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                global_seq,
                event.event_id,
                source_offset,
                source_len,
                event.task_id.0,
                event.aggregate_id,
                event.aggregate_version,
                event.idempotency_key,
                event.event_type,
                event.session_id.as_ref().map(|id| id.0.as_str()),
                event.recorded_at,
                event_json,
            ],
        )
        .map_err(|error| {
            sqlite_error("V3_INDEX_EVENT_CONFLICT")(error)
                .with_detail("event_id", event.event_id.clone())
                .with_detail("idempotency_key", event.idempotency_key.clone())
        })?;
    Ok(())
}

fn refresh_task_shards(
    transaction: &Transaction<'_>,
    project_id: &str,
    task_id: &str,
) -> Result<(), V3Error> {
    let events = query_events_transaction(
        transaction,
        "SELECT event_json FROM events WHERE task_id = ?1 ORDER BY global_seq",
        task_id,
    )?;
    let projection = projection::fold(project_id, &events);
    let last_global_seq = transaction
        .query_row(
            "SELECT COALESCE(MAX(global_seq), 0) FROM events WHERE task_id = ?1",
            params![task_id],
            |row| row.get::<_, u64>(0),
        )
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;

    if let Some(task) = projection.tasks.get(task_id) {
        upsert_projection(
            transaction,
            "task_projections",
            "task_id",
            task_id,
            task_id,
            task,
            last_global_seq,
        )?;
    }
    transaction
        .execute(
            "DELETE FROM worktree_projections WHERE task_id = ?1",
            params![task_id],
        )
        .map_err(sqlite_error("V3_INDEX_UPDATE_FAILED"))?;
    for (worktree_id, worktree) in projection.worktrees {
        upsert_projection(
            transaction,
            "worktree_projections",
            "worktree_id",
            &worktree_id,
            task_id,
            &worktree,
            last_global_seq,
        )?;
    }
    Ok(())
}

fn refresh_session_shards(
    transaction: &Transaction<'_>,
    project_id: &str,
    session_id: &str,
) -> Result<(), V3Error> {
    let events = query_events_transaction(
        transaction,
        "SELECT event_json FROM events WHERE session_id = ?1 ORDER BY global_seq",
        session_id,
    )?;
    let projection = projection::fold(project_id, &events);
    let last_global_seq = transaction
        .query_row(
            "SELECT COALESCE(MAX(global_seq), 0) FROM events WHERE session_id = ?1",
            params![session_id],
            |row| row.get::<_, u64>(0),
        )
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    let fallback_task_id = projection
        .sessions
        .get(session_id)
        .map(|session| session.task_id.as_str())
        .or_else(|| events.first().map(|event| event.task_id.0.as_str()))
        .unwrap_or_default();
    if let Some(session) = projection.sessions.get(session_id) {
        upsert_projection(
            transaction,
            "session_projections",
            "session_id",
            session_id,
            &session.task_id,
            session,
            last_global_seq,
        )?;
    }
    if let Some(binding) = projection.session_bindings.get(session_id) {
        let binding_task_id = binding.bound_task_id.as_deref().unwrap_or(fallback_task_id);
        upsert_projection(
            transaction,
            "session_bindings",
            "session_id",
            session_id,
            binding_task_id,
            binding,
            last_global_seq,
        )?;
    }
    Ok(())
}

fn refresh_memory_shard(transaction: &Transaction<'_>, project_id: &str) -> Result<(), V3Error> {
    let events = query_all_events_transaction(
        transaction,
        "SELECT event_json FROM events WHERE event_type LIKE 'memory.%' ORDER BY global_seq",
    )?;
    let projection = super::project_memory::fold(project_id, &events);
    let last_global_seq = transaction
        .query_row(
            "SELECT COALESCE(MAX(global_seq), 0) FROM events WHERE event_type LIKE 'memory.%'",
            [],
            |row| row.get::<_, u64>(0),
        )
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    let json =
        serde_json::to_string(&projection).map_err(json_error("V3_INDEX_SERIALIZE_FAILED"))?;
    transaction
        .execute(
            "INSERT INTO project_memory(project_id, projection_json, last_global_seq)
             VALUES(?1, ?2, ?3)
             ON CONFLICT(project_id) DO UPDATE SET
                projection_json = excluded.projection_json,
                last_global_seq = excluded.last_global_seq",
            params![project_id, json, last_global_seq],
        )
        .map_err(sqlite_error("V3_INDEX_UPDATE_FAILED"))?;
    Ok(())
}

fn upsert_projection<T: serde::Serialize>(
    transaction: &Transaction<'_>,
    table: &str,
    id_column: &str,
    id: &str,
    task_id: &str,
    projection: &T,
    last_global_seq: u64,
) -> Result<(), V3Error> {
    let json =
        serde_json::to_string(projection).map_err(json_error("V3_INDEX_SERIALIZE_FAILED"))?;
    let sql = if table == "task_projections" {
        format!(
            "INSERT INTO {table}({id_column}, projection_json, last_global_seq)
             VALUES(?1, ?2, ?3)
             ON CONFLICT({id_column}) DO UPDATE SET
                projection_json = excluded.projection_json,
                last_global_seq = excluded.last_global_seq"
        )
    } else {
        format!(
            "INSERT INTO {table}({id_column}, task_id, projection_json, last_global_seq)
             VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT({id_column}) DO UPDATE SET
                task_id = excluded.task_id,
                projection_json = excluded.projection_json,
                last_global_seq = excluded.last_global_seq"
        )
    };
    if table == "task_projections" {
        transaction
            .execute(&sql, params![id, json, last_global_seq])
            .map_err(sqlite_error("V3_INDEX_UPDATE_FAILED"))?;
    } else {
        transaction
            .execute(&sql, params![id, task_id, json, last_global_seq])
            .map_err(sqlite_error("V3_INDEX_UPDATE_FAILED"))?;
    }
    Ok(())
}

fn query_events_transaction(
    transaction: &Transaction<'_>,
    sql: &str,
    value: &str,
) -> Result<Vec<V3EventEnvelope>, V3Error> {
    let mut statement = transaction
        .prepare(sql)
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    let rows = statement
        .query_map(params![value], |row| row.get::<_, String>(0))
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    let mut events = Vec::new();
    for row in rows {
        events.push(decode_json(
            &row.map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?,
        )?);
    }
    Ok(events)
}

fn query_all_events_transaction(
    transaction: &Transaction<'_>,
    sql: &str,
) -> Result<Vec<V3EventEnvelope>, V3Error> {
    let mut statement = transaction
        .prepare(sql)
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    let mut events = Vec::new();
    for row in rows {
        events.push(decode_json(
            &row.map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?,
        )?);
    }
    Ok(events)
}

fn query_json_map<T: DeserializeOwned>(
    connection: &Connection,
    sql: &str,
) -> Result<BTreeMap<String, T>, V3Error> {
    let mut statement = connection
        .prepare(sql)
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    let mut values = BTreeMap::new();
    for row in rows {
        let (id, json) = row.map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
        values.insert(id, decode_json(&json)?);
    }
    Ok(values)
}

fn query_strings(connection: &Connection, sql: &str) -> Result<Vec<String>, V3Error> {
    let mut statement = connection
        .prepare(sql)
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?);
    }
    Ok(values)
}

fn max_global_sequence(connection: &Connection) -> Result<u64, V3Error> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(global_seq), 0) FROM events",
            [],
            |row| row.get(0),
        )
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))
}

fn source_file_bytes(events_path: &Path) -> Result<u64, V3Error> {
    match fs::metadata(events_path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(io_error("V3_EVENT_METADATA_FAILED")(error)),
    }
}

fn meta_u64(connection: &Connection, key: &str) -> Result<Option<u64>, V3Error> {
    let value = connection
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sqlite_error("V3_INDEX_QUERY_FAILED"))?;
    value
        .map(|value| {
            value.parse::<u64>().map_err(|error| {
                V3Error::new(
                    "V3_INDEX_META_INVALID",
                    V3ErrorCategory::CorruptLog,
                    false,
                    error.to_string(),
                )
                .with_detail("key", key.to_owned())
            })
        })
        .transpose()
}

fn set_meta(connection: &Connection, key: &str, value: &str) -> Result<(), V3Error> {
    connection
        .execute(
            "INSERT INTO meta(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(sqlite_error("V3_INDEX_UPDATE_FAILED"))?;
    Ok(())
}

fn validate_source_path(path: &Path, label: &str) -> Result<(), V3Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(V3Error::new(
            "V3_INDEX_MIGRATION_SOURCE_SYMLINK",
            V3ErrorCategory::PermissionDenied,
            false,
            format!("{label} must not be a symbolic link"),
        )
        .with_detail("path", path.to_string_lossy().to_string())),
        Ok(metadata) if !metadata.is_file() => Err(V3Error::new(
            "V3_INDEX_MIGRATION_SOURCE_INVALID",
            V3ErrorCategory::Validation,
            false,
            format!("{label} must be a regular file when present"),
        )
        .with_detail("path", path.to_string_lossy().to_string())),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("V3_INDEX_MIGRATION_SOURCE_METADATA_FAILED")(error)),
    }
}

fn validate_migration_target(path: &Path) -> Result<(), V3Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(V3Error::new(
            "V3_INDEX_PATH_SYMLINK",
            V3ErrorCategory::PermissionDenied,
            false,
            "derived index path must not be a symbolic link",
        )
        .with_detail("path", path.to_string_lossy().to_string())),
        Ok(_) => Err(V3Error::new(
            "V3_INDEX_MIGRATION_TARGET_EXISTS",
            V3ErrorCategory::VersionConflict,
            true,
            "derived index target already exists",
        )
        .with_detail("path", path.to_string_lossy().to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("V3_INDEX_MIGRATION_TARGET_METADATA_FAILED")(error)),
    }
}

fn validate_temp_index(path: &Path) -> Result<(), V3Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(V3Error::new(
                "V3_INDEX_MIGRATION_TEMP_INVALID",
                V3ErrorCategory::PermissionDenied,
                false,
                "migration temp index must be a regular file",
            )
            .with_detail("path", path.to_string_lossy().to_string()))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("V3_INDEX_MIGRATION_TEMP_METADATA_FAILED")(error)),
    }
}

fn validate_directory_target(path: &Path) -> Result<(), V3Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(V3Error::new(
                "V3_INDEX_MIGRATION_BACKUP_PATH_INVALID",
                V3ErrorCategory::PermissionDenied,
                false,
                "migration backup path must be a real directory",
            )
            .with_detail("path", path.to_string_lossy().to_string()))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("V3_INDEX_MIGRATION_BACKUP_METADATA_FAILED")(error)),
    }
}

fn source_snapshot(path: &Path, backup_name: &str) -> Result<SourceSnapshot, V3Error> {
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let present = match File::open(path) {
        Ok(mut file) => {
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let read = file
                    .read(&mut buffer)
                    .map_err(io_error("V3_INDEX_MIGRATION_SOURCE_READ_FAILED"))?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
                bytes = bytes.saturating_add(read as u64);
            }
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(io_error("V3_INDEX_MIGRATION_SOURCE_READ_FAILED")(error)),
    };
    Ok(SourceSnapshot {
        present,
        bytes,
        sha256: format!("sha256:{:x}", hasher.finalize()),
        backup_name: backup_name.to_owned(),
    })
}

fn ensure_backup(
    backups_root: &Path,
    events_path: &Path,
    projection_path: &Path,
    manifest: &MigrationManifest,
) -> Result<PathBuf, V3Error> {
    fs::create_dir_all(backups_root)
        .map_err(io_error("V3_INDEX_MIGRATION_BACKUP_CREATE_FAILED"))?;
    validate_directory_target(backups_root)?;
    let mut identity = Sha256::new();
    identity.update(manifest.project_id.as_bytes());
    identity.update(manifest.events.sha256.as_bytes());
    identity.update(manifest.projection.sha256.as_bytes());
    let digest = format!("{:x}", identity.finalize());
    let backup_dir = backups_root.join(format!("store-format-1-{}", &digest[..16]));
    let manifest_path = backup_dir.join("manifest.json");
    if backup_dir.exists() {
        validate_directory_target(&backup_dir)?;
        let existing = fs::read_to_string(&manifest_path)
            .map_err(io_error("V3_INDEX_MIGRATION_BACKUP_READ_FAILED"))?;
        let existing: MigrationManifest = serde_json::from_str(&existing)
            .map_err(json_error("V3_INDEX_MIGRATION_BACKUP_INVALID"))?;
        if existing.project_id != manifest.project_id
            || existing.events != manifest.events
            || existing.projection != manifest.projection
            || existing.source_store_format != manifest.source_store_format
            || existing.target_store_format != manifest.target_store_format
        {
            return Err(V3Error::new(
                "V3_INDEX_MIGRATION_BACKUP_MISMATCH",
                V3ErrorCategory::CorruptLog,
                false,
                "existing migration backup does not match the source snapshot",
            )
            .with_detail("backup_dir", backup_dir.to_string_lossy().to_string()));
        }
        verify_backup_file(&backup_dir, &existing.events)?;
        verify_backup_file(&backup_dir, &existing.projection)?;
        return Ok(backup_dir);
    }

    let temp_dir = backups_root.join(format!(".backup-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&temp_dir).map_err(io_error("V3_INDEX_MIGRATION_BACKUP_CREATE_FAILED"))?;
    copy_backup_file(events_path, &temp_dir, &manifest.events)?;
    copy_backup_file(projection_path, &temp_dir, &manifest.projection)?;
    let content = serde_json::to_vec_pretty(manifest)
        .map_err(json_error("V3_INDEX_MIGRATION_BACKUP_ENCODE_FAILED"))?;
    let mut file = File::create(temp_dir.join("manifest.json"))
        .map_err(io_error("V3_INDEX_MIGRATION_BACKUP_WRITE_FAILED"))?;
    file.write_all(&content)
        .map_err(io_error("V3_INDEX_MIGRATION_BACKUP_WRITE_FAILED"))?;
    file.sync_all()
        .map_err(io_error("V3_INDEX_MIGRATION_BACKUP_SYNC_FAILED"))?;
    fs::rename(&temp_dir, &backup_dir)
        .map_err(io_error("V3_INDEX_MIGRATION_BACKUP_RENAME_FAILED"))?;
    sync_parent(&backup_dir)?;
    Ok(backup_dir)
}

fn copy_backup_file(
    source: &Path,
    backup_dir: &Path,
    snapshot: &SourceSnapshot,
) -> Result<(), V3Error> {
    if !snapshot.present {
        return Ok(());
    }
    let target = backup_dir.join(&snapshot.backup_name);
    fs::copy(source, &target).map_err(io_error("V3_INDEX_MIGRATION_BACKUP_COPY_FAILED"))?;
    File::open(&target)
        .and_then(|file| file.sync_all())
        .map_err(io_error("V3_INDEX_MIGRATION_BACKUP_SYNC_FAILED"))?;
    verify_backup_file(backup_dir, snapshot)
}

fn verify_backup_file(backup_dir: &Path, snapshot: &SourceSnapshot) -> Result<(), V3Error> {
    let target = backup_dir.join(&snapshot.backup_name);
    if !snapshot.present {
        if target.exists() {
            return Err(V3Error::new(
                "V3_INDEX_MIGRATION_BACKUP_MISMATCH",
                V3ErrorCategory::CorruptLog,
                false,
                "backup unexpectedly contains a source file that was absent",
            ));
        }
        return Ok(());
    }
    let observed = source_snapshot(&target, &snapshot.backup_name)?;
    if observed.bytes != snapshot.bytes || observed.sha256 != snapshot.sha256 {
        return Err(V3Error::new(
            "V3_INDEX_MIGRATION_BACKUP_MISMATCH",
            V3ErrorCategory::CorruptLog,
            false,
            "migration backup hash does not match the source snapshot",
        )
        .with_detail("backup_path", target.to_string_lossy().to_string()));
    }
    Ok(())
}

fn write_migration_marker(
    marker_path: &Path,
    manifest: &MigrationManifest,
    index_path: &Path,
) -> Result<(), V3Error> {
    marker_path.parent().ok_or_else(|| {
        migration_error(
            "V3_INDEX_MIGRATION_MARKER_PATH_INVALID",
            index_path,
            "migration marker has no parent",
        )
    })?;
    let temp = marker_path.with_extension("json.tmp");
    for path in [marker_path, temp.as_path()] {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(V3Error::new(
                    "V3_INDEX_MIGRATION_MARKER_PATH_INVALID",
                    V3ErrorCategory::PermissionDenied,
                    false,
                    "migration marker paths must not be symlinks or non-files",
                )
                .with_detail("path", path.to_string_lossy().to_string()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(io_error("V3_INDEX_MIGRATION_MARKER_METADATA_FAILED")(error));
            }
        }
    }
    let value = serde_json::json!({
        "schema_version": "1.0",
        "status": "complete",
        "store_format": STORE_FORMAT,
        "model_version": STORE_MODEL_VERSION,
        "project_id": manifest.project_id,
        "source": manifest,
        "index_file": index_path.file_name().and_then(|name| name.to_str()),
    });
    let content = serde_json::to_vec_pretty(&value)
        .map_err(json_error("V3_INDEX_MIGRATION_MARKER_ENCODE_FAILED"))?;
    let mut file =
        File::create(&temp).map_err(io_error("V3_INDEX_MIGRATION_MARKER_WRITE_FAILED"))?;
    file.write_all(&content)
        .map_err(io_error("V3_INDEX_MIGRATION_MARKER_WRITE_FAILED"))?;
    file.sync_all()
        .map_err(io_error("V3_INDEX_MIGRATION_MARKER_SYNC_FAILED"))?;
    fs::rename(&temp, marker_path).map_err(io_error("V3_INDEX_MIGRATION_MARKER_RENAME_FAILED"))?;
    sync_parent(marker_path)
}

fn cleanup_sqlite_sidecars(path: &Path) -> Result<(), V3Error> {
    for suffix in ["-wal", "-shm"] {
        let sidecar = sqlite_sidecar(path, suffix);
        match fs::remove_file(&sidecar) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("V3_INDEX_MIGRATION_SIDECAR_CLEAN_FAILED")(error)),
        }
    }
    Ok(())
}

fn sqlite_sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn quarantine_failed_temp(path: &Path) -> Result<(), V3Error> {
    quarantine_failed_index(path).map(|_| ())
}

fn quarantine_failed_index(path: &Path) -> Result<PathBuf, V3Error> {
    let failed = path.with_extension(format!("failed.{}", uuid::Uuid::new_v4()));
    fs::rename(path, &failed).map_err(io_error("V3_INDEX_MIGRATION_TEMP_QUARANTINE_FAILED"))?;
    for suffix in ["-wal", "-shm"] {
        let source = sqlite_sidecar(path, suffix);
        if source.exists() {
            let target = sqlite_sidecar(&failed, suffix);
            fs::rename(source, target)
                .map_err(io_error("V3_INDEX_MIGRATION_TEMP_QUARANTINE_FAILED"))?;
        }
    }
    Ok(failed)
}

fn migration_error(code: &str, index_path: &Path, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Internal, true, message)
        .with_detail("index_path", index_path.to_string_lossy().to_string())
        .with_detail(
            "repair_action",
            "fix the reported path or capacity issue and retry; JSONL/projection sources and the verified backup remain unchanged",
        )
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> Result<(), V3Error> {
    let parent = path.parent().ok_or_else(|| {
        V3Error::new(
            "V3_INDEX_MIGRATION_PARENT_INVALID",
            V3ErrorCategory::Internal,
            false,
            "migration target has no parent",
        )
    })?;
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(io_error("V3_INDEX_MIGRATION_PARENT_SYNC_FAILED"))
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> Result<(), V3Error> {
    Ok(())
}

fn decode_json<T: DeserializeOwned>(json: &str) -> Result<T, V3Error> {
    serde_json::from_str(json).map_err(|error| {
        V3Error::new(
            "V3_INDEX_PROJECTION_INVALID",
            V3ErrorCategory::CorruptLog,
            false,
            error.to_string(),
        )
    })
}

fn sqlite_error(code: &'static str) -> impl FnOnce(rusqlite::Error) -> V3Error {
    move |error| V3Error::new(code, V3ErrorCategory::Internal, true, error.to_string())
}

fn sqlite_busy(error: &V3Error) -> bool {
    error.message.contains("database is locked") || error.message.contains("database is busy")
}

fn io_error(code: &'static str) -> impl FnOnce(std::io::Error) -> V3Error {
    move |error| V3Error::new(code, V3ErrorCategory::Internal, true, error.to_string())
}

fn json_error(code: &'static str) -> impl FnOnce(serde_json::Error) -> V3Error {
    move |error| V3Error::new(code, V3ErrorCategory::Internal, false, error.to_string())
}
