# VibeHub Custom Capability Guide

> Status: Draft v1
> Date: 2026-05-29
> Applies to: workflow schema v2, capability output schema v1.0
> Related: [`RFC-0001`](./rfc/0001-capability-gate-workflow.md), [`Capability Schema v1`](./vibehub-capability-schema-v1.md)

Custom capabilities let a project add deliverable types beyond the built-in set such as `research`, `plan`, `implement`, and `review`. A custom capability participates in the same VibeHub flow as a built-in capability: it can be claimed, gated, given a context pack, schema-checked at output time, released, and included in handoff state.

## When To Add One

Add a custom capability when the work has a repeatable output contract that does not fit a built-in capability. Good examples:

- `security_audit`: required audit scope, findings, and severity summary.
- `migration_review`: required migration target, compatibility notes, and rollback checks.
- `design_critique`: required user journey, issues, and recommendation summary.

Do not add a custom capability only to rename a built-in phase. Keep the built-ins when their output contract already matches the work.

## Declaration

Add `custom_capabilities` to `.vibehub/workflow.yaml`.

```yaml
schema_version: 2
name: example-project

modes:
  evidence_drive:
    capabilities: [research, plan, implement, security_audit, review]

default_mode: evidence_drive

custom_capabilities:
  security_audit:
    required_fields: [audit_scope, "findings[]", severity_summary]
    optional_fields: [recommendations, compliance_refs]
    produces: [audit_report]
    consumes: [diff]
    gates: [security_audit_ready]
    parallel_safe: true

gates:
  security_audit_ready:
    all:
      - has_artifact: diff
      - not:
          open_risk: blocking
```

Rules:

- The custom capability name must be a stable identifier such as `security_audit`.
- It must not conflict with a built-in capability name or an existing workflow capability name.
- Add the custom capability name to each mode that should be allowed to claim it.
- Quote field names ending in `[]` inside flow-style YAML lists, for example `"findings[]"`.

## Field Schema

Custom capability schema is declared through `required_fields` and `optional_fields`.

```yaml
custom_capabilities:
  security_audit:
    required_fields: [audit_scope, "findings[]", severity_summary]
    optional_fields: [recommendations]
```

Validation behavior:

- Every `required_fields` item must be present under the output envelope `data` object.
- Required values must not be `null`.
- A required field ending in `[]` means the JSON output uses the base name as a non-empty array. `"findings[]"` validates `data.findings`.
- Optional fields may be omitted.

Current custom schema support is intentionally simple. Rich JSON Schema typing for each custom field is future scope; use field names and documentation to make expected shapes clear.

## Output Example

Agents or tools write custom capability output with the same envelope as built-ins:

```json
{
  "schema_version": "1.0",
  "capability": "security_audit",
  "task_id": "T-001",
  "run_id": "R-001",
  "created_at": "2026-05-29T00:00:00Z",
  "created_by": "agent",
  "data": {
    "audit_scope": "Authentication changes in the current diff",
    "findings": ["No blocking issue found"],
    "severity_summary": "low",
    "recommendations": "Keep session handling covered by regression tests."
  }
}
```

Invalid examples:

```json
{
  "data": {
    "audit_scope": "Authentication changes",
    "findings": [],
    "severity_summary": "low"
  }
}
```

This fails because `findings` is empty and the workflow declared `"findings[]"`.

```json
{
  "data": {
    "audit_scope": "Authentication changes",
    "findings": ["No issue found"]
  }
}
```

This fails because `severity_summary` is missing.

## Gates And Produced Artifacts

Custom capabilities can use the same gate predicates as built-ins. A completed custom capability contributes its `produces` artifacts to downstream gates.

```yaml
custom_capabilities:
  security_audit:
    required_fields: [audit_scope, "findings[]", severity_summary]
    produces: [audit_report]
    gates: [security_audit_ready]

capabilities:
  review:
    produces: [review_summary]
    gates: [review_ready]

gates:
  security_audit_ready:
    explicit_request: security_audit
  review_ready:
    has_artifact: audit_report
```

In this example, `review` becomes claimable after `security_audit` is released with outcome `completed`.

## Context Packs

When a custom capability is claimed, VibeHub creates a capability-specific context pack. The pack includes a `Capability Output Schema` section with:

- `required_fields`
- `optional_fields`
- `produces`
- `consumes`
- `parallel_safe`
- `custom: true`

Agents should treat that section as the contract for the custom capability output.

## Migration From Preset-Only v1

1. Keep existing built-in capabilities in `capabilities`.
2. Add custom capabilities under `custom_capabilities`.
3. Add each custom capability to the mode-level `capabilities` list.
4. Move any project-specific output expectations into `required_fields` and `optional_fields`.
5. Add gates only when the capability should wait for explicit evidence or artifacts.
6. Run the VibeHub backend tests or an equivalent fixture before relying on the new flow.

Backward compatibility: projects without `custom_capabilities` keep the v1 behavior.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| Unknown capability | Name is not built in and not listed under `custom_capabilities` | Add it to `custom_capabilities` and the mode capability list |
| Name conflict | Custom name matches a built-in or existing workflow capability | Rename the custom capability |
| Missing required field | Output `data` does not include a required key | Add the field to the output |
| Empty array rejected | Field was declared as `"field[]"` | Provide at least one item |
| Gate remains closed | Required artifact, explicit request, or risk condition is not satisfied | Inspect gate reasons with VibeHub status/gates output |
