# RFC Backlog 003: Project Intelligence Lifecycle

- Status: M0 view boundary frozen; M3 lifecycle decision backlog
- Owner: Project Intelligence owner
- Decision milestone: M0 boundary; M3 evidence freeze
- Implementation milestone: M3
- Related task: `T-20260711080143-e975f3c8`
- Evidence: `.vibehub/research/current/research-pack.md` F001, F002, F005

## Problem

The current scanner exposes a depth-3, 12-children-per-directory snapshot. V3 must provide a truthful architecture and file model for large, heterogeneous repositories without presenting stale inference as fact or requiring agents to maintain the map manually.

## Invariants

1. Filesystem, Git, manifest, import/symbol, declared docs, and inferred summaries remain distinguishable evidence layers.
2. Every inferred claim carries evidence refs, confidence, generator version, and freshness.
3. Unsupported or partial analysis degrades to hard file facts.
4. Indexes and views are rebuildable; source files are never modified by the indexer.
5. Paging/search results are stable under an explicit project model version.

## Decisions To Resolve

### D1. Project model identity

Define project/model/version/fingerprint fields, source commit/worktree identity, scan configuration digest, and lifecycle timestamps.

### D2. Lifecycle state machine

Freeze transitions among `unavailable`, `rebuilding`, `fresh`, `stale`, `partial`, `unsupported`, and `error`, including which states may serve cached data.

### D3. Invalidation graph

Specify which file/Git/manifest changes invalidate tree pages, dependencies, symbols, declared knowledge, inferred summaries, and historical links. Structural changes must not force unrelated AI recomputation.

### D4. First analyzers

Choose the first language/build-system set from VibeHub plus two sample repositories. Record unsupported behavior rather than promising universal parsing.

### D5. Scale budgets

Set first-interactive, full-index, incremental-refresh, memory, page-size, search-latency, and persisted-index budgets from measured samples.

### D6. Evidence and UI semantics

Define module/file/edge evidence refs, confidence vocabulary, stale badges, contradictory evidence, and IDE location accuracy.

## Required Spikes

1. Measure current and proposed scans on VibeHub, a monorepo sample, and a non-Rust/TypeScript sample.
2. Change one manifest, one source import, one README, and one unrelated file; observe invalidation scope.
3. Interrupt indexing and verify that no complete/fresh projection is published.

## Deliverables

- Project model and evidence schemas behind `ProjectOverviewView`/`ProjectStructureView`.
- Lifecycle and invalidation state machines.
- Analyzer registry and unsupported/degraded contract.
- Scale budgets and deterministic benchmark corpus.

## Exit Criteria

- M3 can add analyzers without changing M1 view semantics.
- Every architecture relationship can navigate to evidence or state that evidence is unavailable.
- Partial, stale, unsupported, and interrupted indexes are distinguishable in contracts and fixtures.
- Budgets are numeric and measured on macOS and Windows where filesystem behavior matters.

## M0 Freeze Record

- `decided`: structure nodes preserve native/display path identity, source kind,
  confidence, evidence, model version, paging, truncation, analyzer gaps, and
  lifecycle states without exposing scanner internals.
- `decided`: stale, partial, unsupported, interrupted, inaccessible, and empty
  are separate consumer-visible states.
- `deferred`: fingerprints, invalidation graph, analyzer registry, and numeric
  performance budgets require the M3 benchmark corpus.
- Alternatives retained: hard filesystem/Git facts remain available when
  parsers or inference are unsupported; no universal analyzer is assumed.
- Evidence/exit: FX-NO-DOCS, FX-STALE, FX-PARTIAL, FX-WIN-PATHS,
  FX-MAC-PATHS, and FX-LARGE exercise the frozen boundary; native operations
  remain not tested in M0.
