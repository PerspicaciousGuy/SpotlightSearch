# Contributing to Gleam

Thanks for taking an interest. Gleam is a small project with one maintainer, so please keep changes focused and expect replies on a best-effort basis.

## Before you start

- For anything larger than a small fix, open an issue first so we can agree on the direction. Performance is the main constraint: keep the shortcut response, typing and opening results fast before adding visual effects or features.
- Gleam only targets Windows. The core search code (`src-tauri/src/lib.rs`) and the browser preview also run on Linux and macOS, which is how most of it is tested, but the app itself, the shell icons, Store app listing and the Acrylic blur are Windows-only.

## Setup

Prerequisites are in the [README](README.md#build-from-source). Then:

```powershell
npm ci
npm run dev       # browser preview at http://localhost:1420/?demo=1, no system access
npm run desktop   # the real app (Windows)
```

## Checks to run before a pull request

```sh
npm run build                                          # type-check and build the UI
cargo test --manifest-path src-tauri/Cargo.toml --lib  # search, indexing and icon-cache tests
cargo check --manifest-path src-tauri/Cargo.toml --features desktop   # needs the Tauri prerequisites for your OS
```

The [Windows build workflow](https://github.com/PerspicaciousGuy/gleam/actions/workflows/windows.yml) runs the tests and builds the installer on every push and pull request.

## Guidelines

- Match the surrounding code: naming, comment density and idiom. Comments explain why, not what.
- Add or update tests for behaviour you change. Rust tests must not assert timings.
- If a change affects search or indexing speed, measure it with the benchmarks in [PERFORMANCE.md](PERFORMANCE.md) (`search_bench`, `scan_bench`), say what you measured and on what machine, and update the document. Do not claim a Windows result you did not measure on Windows.
- UI changes: include a screenshot, and check keyboard use. Everything should work without the mouse.
- Windows-only code cannot be tested on other systems. Say in the pull request what you ran and where.
- Keep the privacy promises in the README true: no network requests, no telemetry, no reading of file contents.

## Pull requests

Describe what changed and why, how you tested it, and anything you could not test. Small, single-purpose pull requests are reviewed faster.

By contributing you agree that your contribution is licensed under the [MIT License](LICENSE).
