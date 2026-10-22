# Changelog

All notable changes are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - unreleased

First public version.

### Added

- Local SQLite index of file, folder, app and Start Menu shortcut names, with a trigram index for substring search.
- Microsoft Store (MSIX) app search and launch through `Get-StartApps`.
- Alt + Space launcher with category tabs, details pane, copy-path button and tray menu.
- Real app icons from the Windows shell, cached on disk.
- Slash commands (`/a`, `/f`, `/o`, `/r`), a category chip and recent searches.
- Optional Windows Acrylic blur, switchable from the tray menu.
- Hide on focus loss.
- Browser preview with labelled sample files (`npm run dev`).

### Performance

- Prefix and substring searches stop after 40 results instead of sorting every match (1M-entry synthetic benchmark: p95 from about 358 ms to under 1 ms; see PERFORMANCE.md).
- Apps rank ahead of files and folders within each match tier, using a small partial index.
- Single-statement indexing and `synchronous=NORMAL` on the writer connection.

### Known limitations

- The installer is not code-signed.
- No live file watching: the index refreshes on startup and manually.
- Developed and tested on Windows 11 only.
