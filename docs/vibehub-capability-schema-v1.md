# VibeHub Capability Schema v1

> Status: Draft v1
> Date: 2026-05-28
> Baseline refs: §3.3, §7, §17.5, §20, §25
> Depends on: [`RFC-0001`](./rfc/0001-capability-gate-workflow.md)

## 1. Scope

This document expands baseline §7.2 into implementation-ready schema rules for v1 preset capabilities:

- `align_lite`
- `align`
- `research`
- `plan`
- `implement`
- `validate`
- `review_lite`
- `review`

The schema governs capability output files under `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/<capability>.json` and the `capability_goal.required_output_schema` field in Context Packs described by baseline §17.5.

## 2. Shared Envelope

Every capability output uses the same envelope:

| Field | Required | Type | Constraint |
|---|---|---|---|
| `schema_version` | yes | string | exactly `"1.0"` |
| `capability` | yes | string | one of the v1 capability names |
| `task_id` | yes | string | pattern `^T-[A-Za-z0-9_-]+$` |
| `run_id` | yes | string | pattern `^R-[A-Za-z0-9_-]+$` |
| `created_at` | yes | string | ISO 8601 datetime |
| `created_by` | yes | string | min length 1 |
| `data` | yes | object | capability-specific payload |

JSON Schema envelope:

```json
{
  "type": "object",
  "required": ["schema_version", "capability", "task_id", "run_id", "created_at", "created_by", "data"],
  "properties": {
    "schema_version": { "const": "1.0" },
    "capability": { "type": "string" },
    "task_id": { "type": "string", "pattern": "^T-[A-Za-z0-9_-]+$" },
    "run_id": { "type": "string", "pattern": "^R-[A-Za-z0-9_-]+$" },
    "created_at": { "type": "string", "format": "date-time" },
    "created_by": { "type": "string", "minLength": 1 },
    "data": { "type": "object" }
  },
  "additionalProperties": false
}
```

Rust envelope draft:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityOutput<T> {
    pub schema_version: String,
    pub capability: CapabilityName,
    pub task_id: String,
    pub run_id: String,
    pub created_at: String,
    pub created_by: String,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityName {
    AlignLite,
    Align,
    Research,
    Plan,
    Implement,
    Validate,
    ReviewLite,
    Review,
}
```

## 3. Placeholder Constants

Placeholders are allowed only where this table says so. They are explicit values, not missing data.

| Constant | Allowed fields | Meaning |
|---|---|---|
| `"no_risk"` | `risks[].note`, `risk_review`, `concerns[].note` | Agent inspected risk surface and found none |
| `"none"` | optional string fields, `rollback_plan`, `perf_notes` | No applicable content |
| `"n/a"` | `references[]`, `related_threads[]` | Not applicable after inspection |
| `"not_run"` | `test_results.status`, `lint_results.status` | Validation was intentionally not run and must explain why |
| `"unknown"` | `coverage.summary` | Metric unavailable |

Arrays may not be empty when required. If there is no substantive item, include a single placeholder object with an explanatory note.

## 4. Version and Migration Strategy

- Current capability schema version is `"1.0"`.
- Additive optional fields keep the same major version and may increment a document-only minor revision.
- Breaking changes must bump major, for example `"2.0"`.
- Migrations must be deterministic and registered before writes using the new version are accepted.
- Old outputs are never edited in place; migration creates a new output or projection event.

## 5. Shared Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    pub kind: String,
    pub ref_value: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskItem {
    pub id: String,
    pub severity: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRun {
    pub command: String,
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concern {
    pub severity: String,
    pub note: String,
    pub file: Option<String>,
}
```

## 6. `align_lite`

Field table:

| Field | Required | Type | Constraint |
|---|---|---|---|
| `intent` | yes | string | min length 10 |
| `scope` | yes | string | min length 10 |
| `references` | no | string[] | each min length 1 or `"n/a"` |

JSON Schema snippet:

```json
{
  "$id": "vibehub.capability.align_lite.v1",
  "type": "object",
  "required": ["intent", "scope"],
  "properties": {
    "intent": { "type": "string", "minLength": 10 },
    "scope": { "type": "string", "minLength": 10 },
    "references": { "type": "array", "items": { "type": "string", "minLength": 1 } }
  },
  "additionalProperties": true
}
```

Rust struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignLiteOutput {
    pub intent: String,
    pub scope: String,
    pub references: Option<Vec<String>>,
}
```

Valid example:

```json
{ "intent": "Confirm capability refactor direction", "scope": "Document-only S0/S1 alignment", "references": ["docs/vibehub-capability-redesign-2026-05-27.md"] }
```

Invalid example:

```json
{ "intent": "refactor" }
```

Expected error: `schema.required.missing` for `scope`; `schema.type.mismatch` or `schema.required.missing` may also report `intent` if minimum length is enforced as a required-quality error.

## 7. `align`

Field table:

| Field | Required | Type | Constraint |
|---|---|---|---|
| `intent` | yes | string | min length 20 |
| `scope` | yes | string | min length 20 |
| `success_criteria` | yes | string[] | min items 1, each min length 5 |
| `non_goals` | yes | string[] | min items 1, may contain `"none"` only with explanation elsewhere |
| `stakeholders` | no | string[] | each min length 1 |
| `references` | no | string[] | each min length 1 or `"n/a"` |

JSON Schema snippet:

```json
{
  "$id": "vibehub.capability.align.v1",
  "type": "object",
  "required": ["intent", "scope", "success_criteria", "non_goals"],
  "properties": {
    "intent": { "type": "string", "minLength": 20 },
    "scope": { "type": "string", "minLength": 20 },
    "success_criteria": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 5 } },
    "non_goals": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
    "stakeholders": { "type": "array", "items": { "type": "string", "minLength": 1 } },
    "references": { "type": "array", "items": { "type": "string", "minLength": 1 } }
  },
  "additionalProperties": true
}
```

Rust struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignOutput {
    pub intent: String,
    pub scope: String,
    pub success_criteria: Vec<String>,
    pub non_goals: Vec<String>,
    pub stakeholders: Option<Vec<String>>,
    pub references: Option<Vec<String>>,
}
```

Valid example:

```json
{ "intent": "Replace rigid phase order with capability workflow", "scope": "M1-M5 VibeHub backend and agent protocol", "success_criteria": ["Old phase commands remain aliases"], "non_goals": ["No multi-user collaboration in v1"] }
```

Invalid example:

```json
{ "intent": "Replace phase order", "scope": "backend" }
```

Expected error: `schema.required.missing` for `success_criteria` and `non_goals`; `schema.type.mismatch` if `scope` fails minimum-length validation.

## 8. `research`

Field table:

| Field | Required | Type | Constraint |
|---|---|---|---|
| `sources` | yes | object[] | min items 1, each has `kind`, `ref_value`, `summary` |
| `risks` | yes | object[] | min items 1, each has `id`, `severity`, `note`; use `"no_risk"` only as `note` |
| `open_questions` | yes | string[] | min items 1, may contain `"none"` when no questions remain |
| `hypothesis` | no | string | min length 1 |
| `references` | no | string[] | each min length 1 or `"n/a"` |
| `related_threads` | no | string[] | each min length 1 or `"n/a"` |

JSON Schema snippet:

```json
{
  "$id": "vibehub.capability.research.v1",
  "type": "object",
  "required": ["sources", "risks", "open_questions"],
  "properties": {
    "sources": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["kind", "ref_value", "summary"],
        "properties": {
          "kind": { "type": "string", "enum": ["file", "doc", "web", "command", "user_note"] },
          "ref_value": { "type": "string", "minLength": 1 },
          "summary": { "type": "string", "minLength": 10 }
        }
      }
    },
    "risks": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["id", "severity", "note"],
        "properties": {
          "id": { "type": "string", "minLength": 1 },
          "severity": { "type": "string", "enum": ["low", "medium", "high", "blocker"] },
          "note": { "type": "string", "minLength": 1 }
        }
      }
    },
    "open_questions": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
    "hypothesis": { "type": "string", "minLength": 1 },
    "references": { "type": "array", "items": { "type": "string", "minLength": 1 } },
    "related_threads": { "type": "array", "items": { "type": "string", "minLength": 1 } }
  },
  "additionalProperties": true
}
```

Rust struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchOutput {
    pub sources: Vec<SourceRef>,
    pub risks: Vec<RiskItem>,
    pub open_questions: Vec<String>,
    pub hypothesis: Option<String>,
    pub references: Option<Vec<String>>,
    pub related_threads: Option<Vec<String>>,
}
```

Valid example:

```json
{ "sources": [{ "kind": "doc", "ref_value": "docs/vibehub-capability-redesign-2026-05-27.md#7", "summary": "Baseline defines required fields per capability." }], "risks": [{ "id": "R1", "severity": "low", "note": "Schema placeholders still need implementation constants." }], "open_questions": ["Should draft-mode writes be allowed?"] }
```

Invalid example:

```json
{ "sources": [], "risks": [], "open_questions": [] }
```

Expected error: `schema.required.missing` because required arrays cannot be empty.

## 9. `plan`

Field table:

| Field | Required | Type | Constraint |
|---|---|---|---|
| `steps` | yes | object[] | min items 1, each has `id`, `description`, `status` |
| `validation_plan` | yes | string | min length 10 |
| `affected_files` | yes | string[] | min items 1; use `"none"` only for document-only work |
| `alternatives` | no | string[] | min items 1 when present |
| `rollback_plan` | no | string | min length 1 or `"none"` |

JSON Schema snippet:

```json
{
  "$id": "vibehub.capability.plan.v1",
  "type": "object",
  "required": ["steps", "validation_plan", "affected_files"],
  "properties": {
    "steps": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["id", "description", "status"],
        "properties": {
          "id": { "type": "string", "pattern": "^[A-Za-z0-9_.-]+$" },
          "description": { "type": "string", "minLength": 10 },
          "status": { "type": "string", "enum": ["pending", "in_progress", "done", "blocked"] }
        }
      }
    },
    "validation_plan": { "type": "string", "minLength": 10 },
    "affected_files": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
    "alternatives": { "type": "array", "items": { "type": "string", "minLength": 1 } },
    "rollback_plan": { "type": "string", "minLength": 1 }
  },
  "additionalProperties": true
}
```

Rust struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanOutput {
    pub steps: Vec<PlanStep>,
    pub validation_plan: String,
    pub affected_files: Vec<String>,
    pub alternatives: Option<Vec<String>>,
    pub rollback_plan: Option<String>,
}
```

Valid example:

```json
{ "steps": [{ "id": "S2", "description": "Write capability schema document", "status": "done" }], "validation_plan": "Parse the markdown and verify all capability sections exist.", "affected_files": ["docs/vibehub-capability-schema-v1.md"] }
```

Invalid example:

```json
{ "steps": [{ "id": "S2", "description": "Write doc" }], "affected_files": [] }
```

Expected error: `schema.required.missing` for `validation_plan` and step `status`; `schema.required.missing` for empty `affected_files`.

## 10. `implement`

Field table:

| Field | Required | Type | Constraint |
|---|---|---|---|
| `diff_summary` | yes | string | min length 20 |
| `changed_files` | yes | string[] | min items 1 |
| `commands_run` | yes | object[] | may be placeholder command object if no command was needed |
| `manual_test_notes` | no | string | min length 1 or `"none"` |

JSON Schema snippet:

```json
{
  "$id": "vibehub.capability.implement.v1",
  "type": "object",
  "required": ["diff_summary", "changed_files", "commands_run"],
  "properties": {
    "diff_summary": { "type": "string", "minLength": 20 },
    "changed_files": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
    "commands_run": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["command", "status", "summary"],
        "properties": {
          "command": { "type": "string", "minLength": 1 },
          "status": { "type": "string", "enum": ["passed", "failed", "not_run"] },
          "summary": { "type": "string", "minLength": 1 }
        }
      }
    },
    "manual_test_notes": { "type": "string", "minLength": 1 }
  },
  "additionalProperties": true
}
```

Rust struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementOutput {
    pub diff_summary: String,
    pub changed_files: Vec<String>,
    pub commands_run: Vec<CommandRun>,
    pub manual_test_notes: Option<String>,
}
```

Valid example:

```json
{ "diff_summary": "Added RFC and schema documentation for capability workflow.", "changed_files": ["docs/rfc/0001-capability-gate-workflow.md"], "commands_run": [{ "command": "wc -l docs/rfc/0001-capability-gate-workflow.md", "status": "passed", "summary": "RFC is 93 lines." }] }
```

Invalid example:

```json
{ "diff_summary": "changed docs", "changed_files": [] }
```

Expected error: `schema.required.missing` for `commands_run` and non-empty `changed_files`.

## 11. `validate`

Field table:

| Field | Required | Type | Constraint |
|---|---|---|---|
| `test_results` | yes | object | has `status`, `summary`, `commands` |
| `lint_results` | yes | object | has `status`, `summary`, `commands` |
| `status` | yes | string | enum `passed`, `failed`, `blocked` |
| `coverage` | no | object | `summary` string or `"unknown"` |
| `perf_notes` | no | string | min length 1 or `"none"` |

JSON Schema snippet:

```json
{
  "$id": "vibehub.capability.validate.v1",
  "type": "object",
  "required": ["test_results", "lint_results", "status"],
  "properties": {
    "test_results": { "$ref": "#/$defs/run_result" },
    "lint_results": { "$ref": "#/$defs/run_result" },
    "status": { "type": "string", "enum": ["passed", "failed", "blocked"] },
    "coverage": { "type": "object", "required": ["summary"], "properties": { "summary": { "type": "string", "minLength": 1 } } },
    "perf_notes": { "type": "string", "minLength": 1 }
  },
  "$defs": {
    "run_result": {
      "type": "object",
      "required": ["status", "summary", "commands"],
      "properties": {
        "status": { "type": "string", "enum": ["passed", "failed", "not_run"] },
        "summary": { "type": "string", "minLength": 1 },
        "commands": { "type": "array", "items": { "type": "string", "minLength": 1 } }
      }
    }
  },
  "additionalProperties": true
}
```

Rust struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub status: String,
    pub summary: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateOutput {
    pub test_results: RunResult,
    pub lint_results: RunResult,
    pub status: String,
    pub coverage: Option<CoverageSummary>,
    pub perf_notes: Option<String>,
}
```

Valid example:

```json
{ "test_results": { "status": "not_run", "summary": "Document-only S2 change.", "commands": [] }, "lint_results": { "status": "passed", "summary": "Markdown structure checked with rg.", "commands": ["rg -n \"^##\" docs/vibehub-capability-schema-v1.md"] }, "status": "passed", "coverage": { "summary": "unknown" } }
```

Invalid example:

```json
{ "test_results": { "status": "passed" }, "status": "green" }
```

Expected error: `schema.required.missing` for `lint_results` and `test_results.summary`; `schema.type.mismatch` for invalid `status`.

## 12. `review_lite`

Field table:

| Field | Required | Type | Constraint |
|---|---|---|---|
| `summary` | yes | string | min length 20 |
| `concerns` | yes | object[] | min items 1, use `"no_risk"` note when clean |
| `gate_pass` | yes | boolean | true only when concerns are non-blocking |

JSON Schema snippet:

```json
{
  "$id": "vibehub.capability.review_lite.v1",
  "type": "object",
  "required": ["summary", "concerns", "gate_pass"],
  "properties": {
    "summary": { "type": "string", "minLength": 20 },
    "concerns": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["severity", "note"],
        "properties": {
          "severity": { "type": "string", "enum": ["info", "low", "medium", "high", "blocker"] },
          "note": { "type": "string", "minLength": 1 },
          "file": { "type": ["string", "null"] }
        }
      }
    },
    "gate_pass": { "type": "boolean" }
  },
  "additionalProperties": true
}
```

Rust struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewLiteOutput {
    pub summary: String,
    pub concerns: Vec<Concern>,
    pub gate_pass: bool,
}
```

Valid example:

```json
{ "summary": "Reviewed documentation-only capability schema changes.", "concerns": [{ "severity": "info", "note": "no_risk", "file": null }], "gate_pass": true }
```

Invalid example:

```json
{ "summary": "Looks ok", "gate_pass": true }
```

Expected error: `schema.required.missing` for `concerns`; `schema.type.mismatch` if `summary` fails minimum length.

## 13. `review`

Field table:

| Field | Required | Type | Constraint |
|---|---|---|---|
| `summary` | yes | string | min length 20 |
| `concerns` | yes | object[] | min items 1, use `"no_risk"` note when clean |
| `gate_pass` | yes | boolean | false if blocking concerns remain |
| `risk_review` | yes | string | min length 1 or `"no_risk"` |
| `suggestions` | no | string[] | min items 1 when present |
| `follow_ups` | no | string[] | min items 1 when present |

JSON Schema snippet:

```json
{
  "$id": "vibehub.capability.review.v1",
  "type": "object",
  "required": ["summary", "concerns", "gate_pass", "risk_review"],
  "properties": {
    "summary": { "type": "string", "minLength": 20 },
    "concerns": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["severity", "note"],
        "properties": {
          "severity": { "type": "string", "enum": ["info", "low", "medium", "high", "blocker"] },
          "note": { "type": "string", "minLength": 1 },
          "file": { "type": ["string", "null"] }
        }
      }
    },
    "gate_pass": { "type": "boolean" },
    "risk_review": { "type": "string", "minLength": 1 },
    "suggestions": { "type": "array", "items": { "type": "string", "minLength": 1 } },
    "follow_ups": { "type": "array", "items": { "type": "string", "minLength": 1 } }
  },
  "additionalProperties": true
}
```

Rust struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewOutput {
    pub summary: String,
    pub concerns: Vec<Concern>,
    pub gate_pass: bool,
    pub risk_review: String,
    pub suggestions: Option<Vec<String>>,
    pub follow_ups: Option<Vec<String>>,
}
```

Valid example:

```json
{ "summary": "Full review found no blocking schema issues.", "concerns": [{ "severity": "info", "note": "no_risk", "file": null }], "gate_pass": true, "risk_review": "no_risk", "follow_ups": ["Implement Rust schema_check.rs in M5."] }
```

Invalid example:

```json
{ "summary": "Full review found no blocking schema issues.", "concerns": [], "gate_pass": true }
```

Expected error: `schema.required.missing` for `risk_review`; `schema.required.missing` for empty `concerns`.

## 14. Field Relationship Rules

- `review.gate_pass` and `review_lite.gate_pass` must be `false` when any concern has `severity = "blocker"`.
- `validate.status` is `passed` only when `test_results.status` and `lint_results.status` are `passed` or explicitly justified as `not_run`.
- `implement.changed_files` must be consistent with observed diff files unless the work is documentation-only and the diff confirms that scope.
- `research.risks` and `review.concerns` must not be empty; use explicit placeholder objects to show inspection happened.
- `plan.affected_files` may use `["none"]` only for pure process/document decisions.

## 15. Error Codes

All validation errors must use baseline §25.1 codes:

| Error | When to use |
|---|---|
| `schema.required.missing` | Required field missing, required array empty, or required nested field missing |
| `schema.type.mismatch` | Type, enum, pattern, or minimum length violation |
| `schema.version.unsupported` | `schema_version` is absent or unsupported |
| `schema.relationship.invalid` | Field relationship rule fails |

Each error response must include `message`, `hint`, and `details.schema_ref` as described in baseline §25.2.
