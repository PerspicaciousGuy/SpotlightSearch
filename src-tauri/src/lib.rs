use rusqlite::{params, Connection};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use walkdir::WalkDir;

#[derive(Clone, Serialize, Debug)]
pub struct SearchResult {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub kind: String,
}
#[derive(Serialize)]
pub struct IndexStatus {
    pub indexing: bool,
    pub indexed: u64,
    pub skipped: u64,
    pub error: Option<String>,
}

pub struct Index {
    db: PathBuf,
    reader: Mutex<Connection>,
    running: AtomicBool,
    count: AtomicU64,
    skipped: AtomicU64,
    error: Mutex<Option<String>>,
}

type Result<T> = std::result::Result<T, String>;
fn sql<T>(r: rusqlite::Result<T>) -> Result<T> {
    r.map_err(|e| e.to_string())
}

impl Index {
    pub fn new(db: PathBuf) -> Result<Arc<Self>> {
        if let Some(parent) = db.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = sql(Connection::open(&db))?;
        sql(conn.execute_batch("PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS files(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, name_lower TEXT NOT NULL, path TEXT NOT NULL UNIQUE, kind TEXT NOT NULL, generation INTEGER NOT NULL);
            CREATE INDEX IF NOT EXISTS files_name ON files(name_lower COLLATE NOCASE);
            CREATE VIRTUAL TABLE IF NOT EXISTS names USING fts5(name, content='files', content_rowid='id', tokenize='trigram');
            CREATE TRIGGER IF NOT EXISTS files_insert AFTER INSERT ON files BEGIN INSERT INTO names(rowid,name) VALUES(new.id,new.name); END;
            CREATE TRIGGER IF NOT EXISTS files_delete AFTER DELETE ON files BEGIN INSERT INTO names(names,rowid,name) VALUES('delete',old.id,old.name); END;"))?;
        let count = sql(conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get::<_, u64>(0)))?;
        // Keep one bounded page cache and prepared statements warm across keystrokes.
        // The scanner uses a separate writer; WAL readers can search during each batch.
        sql(conn.execute_batch("PRAGMA query_only=ON; PRAGMA cache_size=-2048;"))?;
        sql(conn.busy_timeout(std::time::Duration::from_millis(100)))?;
        conn.set_prepared_statement_cache_capacity(4);
        Ok(Arc::new(Self {
            db,
            reader: Mutex::new(conn),
            running: AtomicBool::new(false),
            count: AtomicU64::new(count),
            skipped: AtomicU64::new(0),
            error: Mutex::new(None),
        }))
    }
    fn connect(&self) -> Result<Connection> {
        let c = sql(Connection::open(&self.db))?;
        sql(c.busy_timeout(std::time::Duration::from_secs(5)))?;
        // WAL defaults to synchronous=FULL, which syncs on every 500-row batch. NORMAL
        // syncs only at checkpoints and cannot corrupt the database; a crash may lose
        // the last batches, which the next refresh re-indexes.
        sql(c.execute_batch("PRAGMA synchronous=NORMAL;"))?;
        Ok(c)
    }
    pub fn status(&self) -> IndexStatus {
        IndexStatus {
            indexing: self.running.load(Ordering::Relaxed),
            indexed: self.count.load(Ordering::Relaxed),
            skipped: self.skipped.load(Ordering::Relaxed),
            error: self.error.lock().unwrap().clone(),
        }
    }
    pub fn rebuild(self: &Arc<Self>, roots: Vec<PathBuf>) {
        self.rebuild_with_roots(move || roots, false);
    }
    pub fn rebuild_system(self: &Arc<Self>) {
        // Probing a disconnected/network drive can block. Do it on the worker too.
        self.rebuild_with_roots(search_roots, true);
    }
    fn rebuild_with_roots<F>(self: &Arc<Self>, roots: F, packaged_apps: bool)
    where
        F: FnOnce() -> Vec<PathBuf> + Send + 'static,
    {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let index = self.clone();
        *index.error.lock().unwrap() = None;
        index.skipped.store(0, Ordering::Relaxed);
        std::thread::spawn(move || {
            if let Err(e) = index.scan_with(roots(), packaged_apps) {
                *index.error.lock().unwrap() = Some(e);
            }
            index.running.store(false, Ordering::SeqCst);
        });
    }
    fn scan(&self, roots: Vec<PathBuf>) -> Result<()> {
        self.scan_with(roots, false)
    }
    fn scan_with(&self, roots: Vec<PathBuf>, packaged_apps: bool) -> Result<()> {
        let mut conn = self.connect()?;
        let generation: i64 = sql(conn.query_row(
            "SELECT COALESCE(MAX(generation),0)+1 FROM files",
            [],
            |r| r.get(0),
        ))?;
        let mut batch = Vec::with_capacity(500);
        if packaged_apps {
            // Indexed first so Store apps are searchable long before the disk walk ends.
            batch.extend(start_apps());
            self.flush(&mut conn, &mut batch, generation)?;
        }
        for root in roots {
            for entry in WalkDir::new(root)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    !e.file_type().is_symlink()
                        && !matches!(
                            e.file_name().to_str(),
                            Some(
                                ".git"
                                    | "node_modules"
                                    | "$Recycle.Bin"
                                    | "System Volume Information"
                            )
                        )
                })
            {
                match entry {
                    Ok(e) if e.depth() > 0 => {
                        let path = e.path().to_string_lossy().into_owned();
                        let name = e.file_name().to_string_lossy().into_owned();
                        let kind = classify(e.path(), e.file_type().is_dir());
                        batch.push((name, path, kind));
                        if batch.len() == 500 {
                            self.flush(&mut conn, &mut batch, generation)?;
                        }
                    }
                    Err(_) => {
                        self.skipped.fetch_add(1, Ordering::Relaxed);
                    }
                    _ => {}
                }
            }
        }
        self.flush(&mut conn, &mut batch, generation)?;
        sql(conn.execute("DELETE FROM files WHERE generation != ?1", [generation]))?;
        self.count.store(
            sql(conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0)))?,
            Ordering::Relaxed,
        );
        Ok(())
    }
    fn flush(
        &self,
        conn: &mut Connection,
        batch: &mut Vec<(String, String, &str)>,
        generation: i64,
    ) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }
        let tx = sql(conn.transaction())?;
        let mut added = 0;
        {
            // One statement per file: new paths insert, known paths only refresh their
            // generation. The FTS triggers fire on insert/delete, not on this update.
            let mut upsert = sql(tx.prepare_cached("INSERT INTO files(name,name_lower,path,kind,generation) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(path) DO UPDATE SET generation=excluded.generation"))?;
            for (name, path, kind) in batch.drain(..) {
                // AUTOINCREMENT ids only grow, and the update path leaves
                // last_insert_rowid alone, so a change means a new row.
                let previous = tx.last_insert_rowid();
                sql(upsert.execute(params![name, name.to_lowercase(), path, kind, generation]))?;
                added += (tx.last_insert_rowid() != previous) as u64;
            }
        }
        sql(tx.commit())?;
        self.count.fetch_add(added, Ordering::Relaxed);
        Ok(())
    }
    pub fn search(&self, query: &str, kind: &str) -> Result<Vec<SearchResult>> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Ok(vec![]);
        }
        if query.chars().count() > 256 {
            return Err("Use a search shorter than 256 characters.".into());
        }
        let conn = self
            .reader
            .lock()
            .map_err(|_| "Search connection is unavailable.".to_string())?;
        let prefix = format!(
            "{}%",
            query
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        // Exact and prefix matches always outrank substring matches. Walk the
        // name_lower index in order instead of sorting every prefix hit by length:
        // an exact name is the smallest string with its own prefix, so it comes
        // first, and SQLite stops after 40 rows however many names share the prefix.
        let mut results = {
            let mut statement = sql(conn.prepare_cached(
                "SELECT f.id,f.name,f.path,f.kind FROM files f
                 WHERE f.name_lower LIKE ?1 ESCAPE '\\' AND (?2='all' OR f.kind=?2)
                 ORDER BY f.name_lower COLLATE NOCASE LIMIT 40",
            ))?;
            let rows = sql(statement.query_map(params![prefix, kind], search_result))?;
            sql(rows.collect::<rusqlite::Result<Vec<_>>>())?
        };
        if results.len() == 40 || query.chars().count() < 3 {
            return Ok(results);
        }
        // FTS emits row IDs in index order. Keep that stable order within the
        // substring tier rather than sorting every hit just to return forty.
        // A common suffix can match millions of names; this lets SQLite stop
        // once the remaining slots are filled without materializing the hit set.
        let term = format!("\"{}\"", query.replace('"', "\"\""));
        let remaining = (40 - results.len()) as i64;
        let mut statement = sql(conn.prepare_cached(
            "SELECT f.id,f.name,f.path,f.kind FROM names
             JOIN files f ON f.id=names.rowid
             WHERE names MATCH ?1 AND (?2='all' OR f.kind=?2)
                   AND f.name_lower NOT LIKE ?3 ESCAPE '\\'
             ORDER BY names.rowid LIMIT ?4",
        ))?;
        let rows = sql(statement.query_map(params![term, kind, prefix, remaining], search_result))?;
        results.extend(sql(rows.collect::<rusqlite::Result<Vec<_>>>())?);
        Ok(results)
    }

    pub fn path_for_id(&self, id: i64) -> Result<PathBuf> {
        let conn = self
            .reader
            .lock()
            .map_err(|_| "Search connection is unavailable.".to_string())?;
        let mut statement = sql(conn.prepare_cached("SELECT path FROM files WHERE id=?1"))?;
        sql(statement.query_row([id], |r| r.get::<_, String>(0))).map(PathBuf::from)
    }
}
fn search_result(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchResult> {
    Ok(SearchResult {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        kind: row.get(3)?,
    })
}

/// Packaged (Store/MSIX) apps have no file on disk the index can walk; they are listed by
/// the shell and launched through this virtual folder, e.g. `shell:AppsFolder\Pkg_id!App`.
pub const APPS_FOLDER: &str = "shell:AppsFolder\\";

/// Parse `Name<TAB>AppID` lines from `Get-StartApps`. Only packaged apps (an AppID with
/// `!`) are kept; classic apps are already indexed from their Start Menu shortcuts.
fn parse_start_apps(output: &str) -> Vec<(String, String, &'static str)> {
    let mut seen = std::collections::HashSet::new();
    output
        .trim_start_matches('\u{feff}')
        .lines()
        .filter_map(|line| {
            let (name, id) = line.trim_end_matches('\r').split_once('\t')?;
            let (name, id) = (name.trim(), id.trim());
            if name.is_empty() || !id.contains('!') || id.contains(char::is_whitespace) {
                return None;
            }
            let path = format!("{APPS_FOLDER}{id}");
            seen.insert(path.clone()).then(|| (name.to_string(), path, "app"))
        })
        .collect()
}

#[cfg(windows)]
fn start_apps() -> Vec<(String, String, &'static str)> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const SCRIPT: &str = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-StartApps | ForEach-Object { $_.Name + [char]9 + $_.AppID }";
    let child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else { return Vec::new() };
    let Some(mut stdout) = child.stdout.take() else { return Vec::new() };
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    // A stuck PowerShell must not keep the indexer marked as running forever.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
        }
    }
    let bytes = reader.join().unwrap_or_default();
    parse_start_apps(&String::from_utf8_lossy(&bytes))
}
#[cfg(not(windows))]
fn start_apps() -> Vec<(String, String, &'static str)> {
    Vec::new()
}

fn classify(path: &Path, folder: bool) -> &'static str {
    if folder {
        return "folder";
    }
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if ["exe", "lnk", "appref-ms"]
        .iter()
        .any(|app| extension.eq_ignore_ascii_case(app))
    {
        "app"
    } else {
        "file"
    }
}
pub fn search_roots() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        ('A'..='Z')
            .map(|c| PathBuf::from(format!("{c}:\\")))
            .filter(|p| p.is_dir())
            .collect()
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::home_dir().into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(label: &str) -> (PathBuf, PathBuf, Arc<Index>) {
        let temp = std::env::temp_dir().join(format!("lumen-{label}-{}", std::process::id()));
        let root = temp.join("files");
        std::fs::create_dir_all(&root).unwrap();
        let index = Index::new(temp.join("index.db")).unwrap();
        (temp, root, index)
    }

    #[test]
    fn start_apps_keep_only_packaged_apps_once() {
        let output = "\u{feff}TradingView\tTradingView.Desktop_n534cwy3pjxzj!TradingView.Desktop\r\n\
            Notepad++\t{6D809377-6AF0-444B-8957-A3773F02200E}\\Notepad++\\notepad++.exe\r\n\
            Microsoft Edge\tMSEdge\r\n\
            日本語 App\tPkg_1!App\r\n\
            Duplicate\tPkg_1!App\r\n\
            \r\nno tab here\r\n";
        let apps = parse_start_apps(output);
        assert_eq!(
            apps,
            vec![
                (
                    "TradingView".to_string(),
                    format!("{APPS_FOLDER}TradingView.Desktop_n534cwy3pjxzj!TradingView.Desktop"),
                    "app"
                ),
                ("日本語 App".to_string(), format!("{APPS_FOLDER}Pkg_1!App"), "app"),
            ]
        );
    }

    #[test]
    fn packaged_apps_are_searchable_by_name() {
        let (temp, _root, index) = fixture("packaged-apps");
        let mut batch = parse_start_apps("TradingView\tPkg_x!App\n");
        index
            .flush(&mut index.connect().unwrap(), &mut batch, 1)
            .unwrap();
        let found = index.search("tradingv", "app").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, "shell:AppsFolder\\Pkg_x!App");
        assert_eq!(index.path_for_id(found[0].id).unwrap(), PathBuf::from(&found[0].path));
        drop(index);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn warm_search_preserves_ranking_and_observes_refreshes() {
        let (temp, root, index) = fixture("warm-search");
        for file in ["budget", "budget.exe", "BUDGET.txt", "my budget.txt"] {
            std::fs::write(root.join(file), "").unwrap();
        }
        for n in 0..60 {
            std::fs::write(root.join(format!("budget extra {n:02}.txt")), "").unwrap();
        }
        index.scan(vec![root.clone()]).unwrap();
        for _ in 0..3 {
            let matches = index.search("BUDGET", "all").unwrap();
            assert_eq!(matches.len(), 40);
            // Exact name first, then prefix matches in case-insensitive name order.
            assert_eq!(matches[0].name, "budget");
            assert_eq!(matches[1].name, "budget extra 00.txt");
            assert!(matches
                .iter()
                .all(|item| item.name.to_lowercase().starts_with("budget")));
            assert!(matches[1..]
                .windows(2)
                .all(|pair| pair[0].name.to_lowercase() <= pair[1].name.to_lowercase()));
            assert_eq!(index.search("budget", "app").unwrap().len(), 1);
        }
        let before = index.search("budget", "app").unwrap()[0].id;
        std::fs::remove_file(root.join("budget.exe")).unwrap();
        std::fs::write(root.join("budget.lnk"), "").unwrap();
        index.scan(vec![root]).unwrap();
        let after = index.search("budget", "app").unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].name, "budget.lnk");
        assert_ne!(after[0].id, before);
        assert!(index.path_for_id(before).is_err());
        drop(index);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn substring_fallback_fills_remaining_slots_in_stable_id_order() {
        let (temp, root, index) = fixture("substring-order");
        let mut names = vec![
            "my budget long name.txt".to_string(),
            "budget ZZZ".to_string(),
            "budget".to_string(),
            "budget a".to_string(),
            "my budget a".to_string(),
        ];
        names.extend((0..60).map(|n| format!("another budget report {n:02}.txt")));
        let mut batch = names
            .into_iter()
            .map(|name| {
                let path = root.join(&name).to_string_lossy().into_owned();
                (name, path, "file")
            })
            .collect();
        index
            .flush(&mut index.connect().unwrap(), &mut batch, 1)
            .unwrap();
        let matches = index.search("budget", "all").unwrap();
        assert_eq!(matches.len(), 40);
        assert_eq!(
            matches[..3]
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            ["budget", "budget a", "budget ZZZ"]
        );
        // Substrings intentionally use persistent index order, not a full-hit-set
        // length sort. Exact/prefix tiers always appear ahead of them.
        assert_eq!(matches[3].name, "my budget long name.txt");
        assert_eq!(matches[4].name, "my budget a");
        assert!(matches[3..].windows(2).all(|pair| pair[0].id < pair[1].id));
        assert_eq!(
            matches
                .iter()
                .map(|item| item.id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            40
        );
        drop(index);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn staged_search_keeps_literal_wildcards_and_unicode_substrings() {
        let (temp, root, index) = fixture("literal-substrings");
        let names = [
            "100% budget.pdf",
            "100X budget.pdf",
            "draft 100% budget.pdf",
            "a_b.docx",
            "aXb.docx",
            "draft a_b.docx",
            "日本語.txt",
            "my 日本語.txt",
            "don't panic.txt",
            "draft don't panic.txt",
        ];
        let mut batch = names
            .into_iter()
            .map(|name| {
                (
                    name.to_string(),
                    root.join(name).to_string_lossy().into_owned(),
                    "file",
                )
            })
            .collect();
        index
            .flush(&mut index.connect().unwrap(), &mut batch, 1)
            .unwrap();
        for query in ["100%", "a_b", "日本語", "don't"] {
            let matches = index.search(query, "file").unwrap();
            assert_eq!(matches.len(), 2, "{query}");
            assert!(matches[0].name.starts_with(query));
            assert!(!matches[1].name.starts_with(query));
            assert!(index.search(query, "app").unwrap().is_empty());
        }
        drop(index);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn concurrent_reads_remain_available_during_index_batches() {
        let (temp, root, index) = fixture("concurrent-search");
        std::fs::write(root.join("budget.txt"), "").unwrap();
        index.scan(vec![root.clone()]).unwrap();
        for n in 0..600 {
            std::fs::write(root.join(format!("budget report {n:03}.txt")), "").unwrap();
        }
        let writer = index.clone();
        let scan = std::thread::spawn(move || writer.scan(vec![root]).unwrap());
        let readers: Vec<_> = (0..4)
            .map(|_| {
                let reader = index.clone();
                std::thread::spawn(move || {
                    for _ in 0..20 {
                        let results = reader.search("budget", "all").unwrap();
                        assert!(!results.is_empty());
                        assert!(results.len() <= 40);
                        assert_eq!(results[0].name, "budget.txt");
                    }
                })
            })
            .collect();
        scan.join().unwrap();
        for reader in readers {
            reader.join().unwrap();
        }
        assert_eq!(index.status().indexed, 601);
        drop(index);
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn scan_search_and_refresh() {
        let temp = std::env::temp_dir().join(format!("lumen-test-{}", std::process::id()));
        let root = temp.join("files");
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        for file in [
            "Budget.xlsx",
            "My Budget notes.txt",
            "100%_done.txt",
            "日本語.txt",
            "Editor.EXE",
            "don't panic.txt",
        ] {
            std::fs::write(root.join(file), "").unwrap();
        }
        std::fs::write(root.join("node_modules/hidden.txt"), "").unwrap();
        let index = Index::new(temp.join("index.db")).unwrap();
        index.scan(vec![root.clone()]).unwrap();
        let results = index.search("budget", "all").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "Budget.xlsx");
        assert_eq!(index.search("udg", "file").unwrap().len(), 2);
        assert_eq!(index.search("bu", "all").unwrap().len(), 1);
        assert_eq!(index.search("editor", "app").unwrap().len(), 1);
        assert_eq!(index.search("editor", "file").unwrap().len(), 0);
        assert_eq!(index.search("100%_", "all").unwrap().len(), 1);
        assert_eq!(index.search("日本語", "all").unwrap().len(), 1);
        assert_eq!(index.search("don't", "all").unwrap().len(), 1);
        assert!(index.search("hidden", "all").unwrap().is_empty());
        assert!(index.search("\" OR *", "all").unwrap().is_empty());
        assert!(index.search("", "all").unwrap().is_empty());
        assert_eq!(
            index.path_for_id(results[0].id).unwrap(),
            root.join("Budget.xlsx")
        );
        std::fs::remove_file(root.join("Budget.xlsx")).unwrap();
        index.scan(vec![root]).unwrap();
        assert_eq!(index.search("budget", "all").unwrap().len(), 1);
        drop(index);
        std::fs::remove_dir_all(temp).unwrap();
    }
}
