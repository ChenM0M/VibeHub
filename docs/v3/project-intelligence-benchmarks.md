# Project Intelligence Benchmarks

Run the release harness five times and use the highest observed value as the conservative p95 for this small sample:

```sh
scripts/v3-project-intelligence/benchmark.sh /path/to/project 5
```

The harness measures the real `ProjectIndexService` first page, full hard-fact index, indexed search, node count, serialized snapshot size, process peak RSS, and peak RSS growth after opening the service. RSS is reported in bytes on Unix; unsupported platforms return `null`. Git ignored files are excluded by `git ls-files --cached --others --exclude-standard`; report a separate visible-file count when evaluating repositories dominated by generated or environment directories.

Frozen M3 budgets are: first page p95 <= 500 ms, indexed search p95 <= 100 ms, VibeHub and ordinary TypeScript repositories full index <= 5 s, large visible-file corpus <= 20 s, peak additional RSS <= 256 MiB, and persisted snapshot <= max(128 MiB, 2x indexed path metadata).

## 2026-07-12 macOS Release Results

Five warmed release runs were recorded per corpus; the table reports the highest observed duration (a conservative p95 for five samples).

| Corpus | Included files | Visible baseline | First page | Full index | Search | Snapshot |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| VibeHub | 850 | 961 | 55.6 ms | 53.8 ms | 60.6 ms | 317,866 B |
| LLMplayground | 508 | 567 | 49.5 ms | 48.1 ms | 55.3 ms | 158,120 B |
| graduation_song_vote | 242 | 188,206 | 54.6 ms | 38.5 ms | 47.1 ms | 73,119 B |

All measured duration and snapshot-size gates pass. The Python corpus difference is intentional evidence: `git ls-files --cached --others --exclude-standard` excludes its environment/generated population.

The current harness now captures peak RSS. On VibeHub, five warmed runs after the indexed-search fix observed first page <= 65.4 ms, full index <= 81.0 ms, indexed search <= 0.7 ms, and peak additional RSS <= 4.2 MiB.

## 200k File Stress Corpus

A generated, committed Rust corpus containing 200,000 files and 202,002 indexed nodes was measured on macOS after warming the filesystem cache. Three runs observed full index <= 4.64 s, indexed search <= 27.0 ms, serialized snapshot 35,680,701 bytes, and peak additional RSS <= 161.8 MiB. These pass the large-corpus, indexed-search, snapshot, and RSS budgets. First page ranged from 350.7 ms to 524.1 ms; the highest sample is 24.1 ms above the 500 ms budget and remains a narrow performance risk rather than a passed gate.
