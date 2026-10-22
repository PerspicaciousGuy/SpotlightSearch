# Lumen

A Windows file launcher built with **Rust, Tauri 2, React, and TypeScript**. Press **Alt + Space**, type a filename, and press **Enter** to open it.

## Run on Windows

Install:

- [Node.js LTS](https://nodejs.org/)
- [Rust through rustup](https://rustup.rs/) using the default MSVC toolchain
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with **Desktop development with C++** and a Windows SDK
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (usually already on Windows 10/11)

Open PowerShell in this project folder:

```powershell
npm ci
npm run desktop
```

The first launch starts indexing in the background. Results appear as batches are saved. A full drive scan can take time; the footer shows progress. Later launches can search the saved index while it refreshes.

- **Alt + Space:** show/hide the launcher
- **Up / Down:** select a result
- **Enter:** open with its Windows default application
- **Escape:** clear the search; press again to hide the launcher
- **Ctrl + 1 / 2 / 3 / 4:** switch All / Apps / Files / Folders
- **Ctrl + L:** select the search field
- **Tray menu:** open, refresh the index, toggle the blur background, or quit
- **Refresh button:** rescan for new, renamed, or removed files

Alt + Space must be available to register as a global shortcut. If another utility owns it, use the tray menu to open Lumen. Closing the window keeps the process running; use **Quit Lumen** in the tray menu to exit.

## Performance comes first

Search runs in Rust against a persistent SQLite index. The UI dispatches immediately with one request in flight and only the latest query pending. Search/status updates pause while the launcher is hidden or unfocused; the Rust indexer can continue its background scan. The surface uses a mostly solid fill; there is no CSS backdrop blur. An experimental native Windows Acrylic blur can be switched off from the tray menu (**Blur background**); see [PERFORMANCE.md](PERFORMANCE.md#acrylic-blur-experiment).

See [performance measurements and remaining work](PERFORMANCE.md). The one-million-entry synthetic Linux benchmark reduced backend p95 from approximately 358 ms to under 1 ms; this is not a Windows end-to-end timing or memory measurement.

## Interface

The launcher opens as a compact search field with category controls. Typing expands it into a results list and a details pane showing the selected item's type and location. Use the copy button to copy its full path. These are filename details; document contents are not read or previewed.

For a browser preview, run `npm run dev`, then open `http://localhost:1420`. Choose **Explore sample files** (or visit `http://localhost:1420/?demo=1`) to try search, filtering, navigation, and copying paths with labeled fixtures. Browser previews do not access your filesystem. The background and preview labels only appear in the browser; the native launcher floats above your Windows desktop.

See [design notes](design/notes.md) and [the updated preview](preview.png). Native window resizing and transparency require Windows validation.

## Build an installer

```powershell
npm run bundle
```

Find the NSIS installer under `src-tauri/target/release/bundle/nsis/`. The installer is unsigned. The included GitHub Actions workflow builds and uploads a Windows installer when the project is pushed to a repository.

## Search behavior and scope

- Searches **names**, not file contents. Includes files, folders, `.exe` apps, and `.lnk`/`.appref-ms` shortcuts, including indexed Start Menu shortcuts.
- Scans accessible mounted drive letters A–Z under your current Windows account. Network/removable drives with letters may be included and may slow indexing. It does not bypass permissions or fetch online-only file contents.
- Does not follow symbolic links or directory junctions. Skips `.git`, `node_modules`, `$Recycle.Bin`, and `System Volume Information` directories.
- Uses a local SQLite FTS5 trigram index for substring matches with **three or more characters**. One- and two-character queries match the beginning of a name. Exact matches rank first, followed by prefixes in case-insensitive name order; remaining substring matches use stable index order. Both tiers read an index in order, so the search stops after filling 40 results.
- Index lives in Tauri's per-user application data directory as `index.sqlite3`. It contains filenames and paths. No network search or telemetry is included.
- Refreshes on startup and manually. **Live filesystem watching, launch at login, settings search, file-content search, and shortcut customization are not implemented yet.** Failed scans retain saved entries; unreadable paths are counted and skipped. A completed refresh removes entries not seen during that scan.

## Development checks

```sh
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml --features desktop
```

The core Rust tests run without a GUI toolchain. The desktop check requires the platform's Tauri prerequisites. `npm run dev` also provides a browser preview, clearly labeled as a preview, without access to system files. Native shortcuts, tray interaction, opening results, and installer behavior must be tested on Windows.
