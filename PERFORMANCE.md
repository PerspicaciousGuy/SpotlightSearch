# Performance

Performance is the primary constraint. Keep shortcut response, typing, and opening results responsive before adding visual effects or richer previews. The launcher stays resident in the tray so opening it does not cold-start the WebView every time. Resident WebView2 memory still needs measurement on Windows.

## What changed

- Removed the fixed 100 ms search debounce. The fast path dispatches immediately.
- Limited frontend search requests to one running request plus one replaceable pending query. Obsolete responses cannot replace current results. The running SQLite query still completes; this is scheduling, not query interruption.
- Stopped frontend polling/search work while hidden, unfocused, or in a hidden browser tab. Active status polls are sequential: 2 seconds during indexing, 10 seconds otherwise. The indexer can still run in the background.
- Reused one SQLite reader with a 2 MiB page-cache target and four cached statements, separate from the WAL writer. This setting is not a total process-memory limit.
- Searched exact/prefix matches first. When those fill 40 slots, substring searching is skipped. Otherwise, a direct FTS join streams remaining matches in row-ID order and stops after filling the list.
- Removed full-hit-set sorting from the prefix tier. Prefix matches now come from the `name_lower` index in name order (an exact name sorts first), so SQLite stops after 40 rows however many names share the prefix. This replaces shortest-name ordering among prefix matches with alphabetical order.
- Removed full-hit-set sorting from the substring tier. This intentionally changes substring ties from shortest-name ordering to stable index order. Exact/prefix priority remains, and prefix ties retain their original ordering.
- Indexed each file with one upsert statement instead of an insert plus a separate update for known paths, reused writer statements across batches, and set `synchronous=NORMAL` on the writer so WAL batches no longer sync on every commit.
- Stopped re-running the active search on every indexing status tick. A status tick now refreshes results only when nothing else is queued, so it cannot discard a search the user is waiting for.
- Moved drive probing and file-opening checks onto workers.
- Removed backdrop blur and the repeating indexing pulse; skipped duplicate native size requests and unchanged UI payload updates.

## Search-core measurements

Release-mode Rust, SQLite's bundled FTS5, Linux cloud environment. Same persisted synthetic fixture before/after; ten queries, 30 warm samples per query. The corpus mixes names, punctuation, Unicode, apps, and common terms. These results exclude WebView rendering, IPC, hotkey registration, cold filesystem caches, and concurrent full-disk indexing. The shared host introduces run-to-run variation.

| Entries | Median before | Median after | p95 before | p95 after |
| --- | ---: | ---: | ---: | ---: |
| 100,000 | 7.37 ms | 5.25 ms | 35.16 ms | 7.73 ms |
| 1,000,000 | 68.04 ms | 56.86 ms | 358.45 ms | 67.47 ms |

After the prefix-tier change, the same benchmark reports (raw: [100k](performance/results/prefix-index-100k.txt), [1m](performance/results/prefix-index-1m.txt)):

| Entries | Median | p95 |
| --- | ---: | ---: |
| 100,000 | 0.035 ms | 0.203 ms |
| 1,000,000 | 0.028 ms | 0.162 ms |

The prefix-heavy queries that took 57–63 ms now take about 0.03 ms. The slowest remaining query is the app-filtered `code` (0.16–0.20 ms). These numbers are from the same synthetic fixture, so a corpus with many more matches in the filtered kind could take longer, because rows are rejected one at a time while walking the index.

Indexer, `scan_bench` on `/usr` (84,410 entries, warm cache, local disk; [raw](performance/results/scan-bench.txt)): first scan 2.51 s to 2.20 s, rescan of the unchanged tree 0.44 s to 0.33–0.36 s. The gain is modest here because the cache is warm and the disk is fast; the `synchronous=NORMAL` benefit should be larger on slow disks, which is unmeasured.

The one-million-row broad substring `2026` dropped from a median 358.45 ms to 0.068 ms because it can fill the result list without sorting every matching filename. Before the prefix-tier change, prefix-heavy queries took 57–63 ms because their ranking examined and sorted every matching prefix. Do not generalize the broad-substring speedup to every query. Index-opening measurements varied; these runs do not establish a startup-time improvement.

Raw results: [100k before](performance/results/before-100k.txt), [100k after](performance/results/after-100k.txt), [1m before](performance/results/before-1m.txt), [1m after](performance/results/after-1m.txt).

Reproduce the current core benchmark with a fresh temporary database path:

```sh
cargo run --release --manifest-path src-tauri/Cargo.toml --example search_bench -- /tmp/lumen-bench.sqlite3 1000000
```

Reproduce the indexer benchmark with a fresh database path and any directory to scan (it reads names only):

```sh
cargo run --release --manifest-path src-tauri/Cargo.toml --example scan_bench -- /tmp/lumen-scan.sqlite3 /usr
```

On Windows, substitute a temporary Windows path, such as `C:\Temp\lumen-bench.sqlite3`. The fixture contains synthetic paths and does not scan or open personal files. It creates a database of approximately 271 MiB at one million entries; repeat with the same size/path to reuse it. Remove the fixture when finished. There are no timing assertions in unit tests.

## Acrylic blur experiment

Lumen can ask Windows 11 for an Acrylic window backdrop (Tauri `set_effects`, backed by the `window-vibrancy` crate). It is on by default and can be turned off at runtime from the tray menu (**Blur background**); the choice is stored as an empty `blur-off` file in the app data directory.

- On Windows 11 build 22523 or newer the crate uses the system backdrop API (`DWMWA_SYSTEMBACKDROP_TYPE`). The crate's README lists a resize/drag performance warning for Acrylic only on Windows 10 v1903+ and Windows 11 build 22000, and for Blur on build 22621; this implementation is neither of those paths on current builds, but **no Windows frame-time, typing or memory measurements exist for it yet**.
- The backdrop covers the whole native window, so blur mode removes the transparent margin around the card and uses the system shadow and rounded corners (`set_shadow(true)`, which Tauri documents as giving undecorated Windows 11 windows rounded corners and a 1px border). The window is forced to the Light theme so the material matches the light UI.
- The tint colour passed to Tauri has no effect on Windows 11 (per Tauri's docs), so legibility comes from the page's translucent fill (currently about 35% white).
- If it feels slow, switch it off from the tray; that restores the solid card, CSS shadow and transparent margin.

## Verification

Five Rust tests cover exact-first ranking, cached-reader refresh visibility, literal punctuation/Unicode, stable substring ordering, deletion, and reads during scan batches. Frontend checks cover keyboard/filter/copy behavior and stale responses. A scheduler stress check submitted 101 requests and executed only the first and latest, with at most one in flight. A mocked-clock check recorded no frontend status/search calls during 35 seconds hidden and confirmed polling resumed on activation. Browser axe checks reported no violations in the tested states; they do not establish complete accessibility compliance.

## Remaining work

- Profile release builds on Windows: hotkey-to-input timing, type-to-result p50/p95, private working set including WebView2 subprocesses, idle CPU, and behavior while indexing HDD/SSD/network drives. No Windows latency or RAM guarantees are established yet.
- Replace repeated full-drive walks with incremental filesystem tracking, with NTFS-specific indexing considered only after Windows measurements. Full scans currently still happen at startup and on manual refresh; unchanged files still require generation updates.
- Measure selective category searches with very broad terms and sparse matches. Filtering occurs after FTS candidate lookup, so these can still traverse many candidates. The result cap bounds returned rows, not all work for every query.
- Consider interrupting long obsolete SQLite queries, and tuning indexer CPU/I/O priority if Windows traces show foreground contention. Preserve correct file filtering and useful ranking when optimizing.
