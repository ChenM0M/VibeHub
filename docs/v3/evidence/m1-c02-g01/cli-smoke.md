# G01 isolated CLI smoke

Platform: macOS
Binary: target/debug/vibehub
Temporary root: redacted isolated /tmp directory

## absent-doctor

```json
{
  "state": "absent",
  "schema_version": null,
  "message": "project does not contain .vibehub",
  "recommended_action": "run `vibehub v3 <project> init` to initialize v3"
}
```

## absent-init

```json
{
  "status": "initialized",
  "archived_legacy_v2": false,
  "created_paths": [
    ".vibehub/project.yaml",
    ".vibehub/events",
    ".vibehub/projections",
    ".vibehub/indexes",
    ".vibehub/runtime"
  ]
}
```

## absent-init-repeat

```json
{
  "status": "already_initialized",
  "archived_legacy_v2": false,
  "created_paths": []
}
```

## absent-doctor-repeat

```json
{
  "state": "v3",
  "schema_version": 3,
  "message": "project uses the v3 disk layout",
  "recommended_action": null
}
```

## v2-doctor

```json
{
  "state": "v2",
  "schema_version": 2,
  "message": "this is a v2 project",
  "recommended_action": "run `vibehub v3 <project> migrate` to archive v2 and initialize v3"
}
```

## v2-migrate

```json
{
  "status": "migrated",
  "archived_legacy_v2": true,
  "created_paths": [
    ".vibehub/project.yaml",
    ".vibehub/events",
    ".vibehub/projections",
    ".vibehub/indexes",
    ".vibehub/runtime",
    ".vibehub/legacy-v2"
  ]
}
```

## v2-doctor-after

```json
{
  "state": "v3",
  "schema_version": 3,
  "message": "project uses the v3 disk layout",
  "recommended_action": null
}
```

## interrupted-back-doctor

```json
{
  "state": "migration_interrupted",
  "schema_version": null,
  "message": "a legacy-v2 migration staging directory is present",
  "recommended_action": "run `vibehub v3 <project> migrate-recover` before retrying migration"
}
```

## interrupted-back-recover

```json
{
  "status": "rolled_back_to_v2",
  "archived_legacy_v2": false,
  "created_paths": []
}
```

## interrupted-back-doctor-after

```json
{
  "state": "v2",
  "schema_version": null,
  "message": "this is a v2 project",
  "recommended_action": "run `vibehub v3 <project> migrate` to archive v2 and initialize v3"
}
```

## interrupted-forward-doctor

```json
{
  "state": "migration_interrupted",
  "schema_version": 3,
  "message": "a legacy-v2 migration staging directory is present",
  "recommended_action": "run `vibehub v3 <project> migrate-recover` before retrying migration"
}
```

## interrupted-forward-recover

```json
{
  "status": "recovered_forward",
  "archived_legacy_v2": true,
  "created_paths": [
    ".vibehub/legacy-v2"
  ]
}
```

## interrupted-forward-doctor-after

```json
{
  "state": "v3",
  "schema_version": 3,
  "message": "project uses the v3 disk layout",
  "recommended_action": null
}
```

## conflict-doctor

```json
{
  "state": "conflict",
  "schema_version": null,
  "message": "legacy-v2 exists without a valid v3 schema marker",
  "recommended_action": "do not overwrite the archive; inspect the project and recover manually"
}
```

## conflict-migrate-refusal

```text
exit_status=1
{
  "code": "V3_MIGRATION_STATE_INVALID",
  "category": "validation",
  "retryable": false,
  "message": "cannot migrate layout state Conflict",
  "details": {}
}
```

## invariants

- V2 payload remained byte-identical under `.vibehub/legacy-v2`.
- Backward recovery restored the staged V2 root.
- Forward recovery archived staged V2 data.
- Conflict migration exited 1 and refused mutation.
- No real repository was used.
