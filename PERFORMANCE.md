# Performance

Performance is the primary constraint. Keep shortcut response, typing, and opening results responsive before adding visual effects or richer previews. The launcher stays resident in the tray so opening it does not cold-start the WebView every time. Resident WebView2 memory still needs measurement on Windows.

## What changed

- Removed the fixed 100 ms search debounce. The fast path dispatches immediately.
- Limited frontend search requests to one running request plus one replaceable pending query. Obsolete responses cannot replace current results. The running SQLite query still completes; this is scheduling, not query interruption.
- Stopped frontend polling/search work while hidden, unfocused, or in a hidden browser tab. Active status polls are sequential: 2 seconds during indexing, 10 seconds otherwise. The indexer can still run in the background.
- Reused one SQLite reader with a 2 MiB page-cache target and four cached statements, separate from the WAL writer. This setting is not a total process-memory limit.
- Searched exact/prefix matches first. When those fill 40 slots, substring searching is skipped. Otherwise, a direct FTS join streams remaining matches in row-ID order and stops after filling the list.
- Removed full-hit-set sorting from the substring tier. This intentionally changes substring ties from shortest-name ordering to stable index order. Exact/prefix priority remains, and prefix ties retain their original ordering.
- Avoided an extra UPDATE for newly indexed files and reused writer statements across batches.
- Moved drive probing and file-opening checks onto workers.
- Removed backdrop blur and the repeating indexing pulse; skipped duplicate native size requests and unchanged UI payload updates.

## Search-core measurements

Release-mode Rust, SQLite's bundled FTS5, Linux cloud environment. Same persisted synthetic fixture before/after; ten queries, 30 warm samples per query. The corpus mixes names, punctuation, Unicode, apps, and common terms. These results exclude WebView rendering, IPC, hotkey registration, cold filesystem caches, and concurrent full-disk indexing. The shared host introduces run-to-run variation.

| Entries | Median before | Median after | p95 before | p95 after |
| --- | ---: | ---: | ---: | ---: |
| 100,000 | 7.37 ms | 5.25 ms | 35.16 ms | 7.73 ms |
| 1,000,000 | 68.04 ms | 56.86 ms | 358.45 ms | 67.47 ms |

The one-million-row broad substring `2026` dropped from a median 358.45 ms to 0.068 ms because it can fill the result list without sorting every matching filename. Prefix-heavy queries remain around 57–63 ms on this fixture because their ranking still examines and sorts matching prefixes. Do not generalize the broad-substring speedup to every query. Index-opening measurements varied; these runs do not establish a startup-time improvement.

Raw results: [100k before](performance/results/before-100k.txt), [100k after](performance/results/after-100k.txt), [1m before](performance/results/before-1m.txt), [1m after](performance/results/after-1m.txt).

Reproduce the current core benchmark with a fresh temporary database path:

```sh
cargo run --release --manifest-path src-tauri/Cargo.toml --example search_bench -- /tmp/lumen-bench.sqlite3 1000000
```

On Windows, substitute a temporary Windows path, such as `C:\Temp\lumen-bench.sqlite3`. The fixture contains synthetic paths and does not scan or open personal files. It creates a database of approximately 271 MiB at one million entries; repeat with the same size/path to reuse it. Remove the fixture when finished. There are no timing assertions in unit tests.

## Verification

Five Rust tests cover ranking, cached-reader refresh visibility, literal punctuation/Unicode, stable substring ordering, deletion, and reads during scan batches. Frontend checks cover keyboard/filter/copy behavior and stale responses. A scheduler stress check submitted 101 requests and executed only the first and latest, with at most one in flight. A mocked-clock check recorded no frontend status/search calls during 35 seconds hidden and confirmed polling resumed on activation. Browser axe checks reported no violations in the tested states; they do not establish complete accessibility compliance.

## Remaining work

- Profile release builds on Windows: hotkey-to-input timing, type-to-result p50/p95, private working set including WebView2 subprocesses, idle CPU, and behavior while indexing HDD/SSD/network drives. No Windows latency or RAM guarantees are established yet.
- Replace repeated full-drive walks with incremental filesystem tracking, with NTFS-specific indexing considered only after Windows measurements. Full scans currently still happen at startup and on manual refresh; unchanged files still require generation updates.
- Measure selective category searches with very broad terms and sparse matches. Filtering occurs after FTS candidate lookup, so these can still traverse many candidates. The result cap bounds returned rows, not all work for every query.
- Consider interrupting long obsolete SQLite queries, improving prefix ranking cost, and tuning indexer CPU/I/O priority if Windows traces show foreground contention. Preserve correct file filtering and useful ranking when optimizing.
