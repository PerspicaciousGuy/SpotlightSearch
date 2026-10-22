//! Reproducible search-core benchmark; no GUI, no user files, no timing assertions.
//! cargo run --release --example search_bench -- [database-path] [entry-count]
use gleam_search::Index;
use rusqlite::{params, Connection};
use std::{path::PathBuf, time::Instant};

fn main() {
    let mut args = std::env::args().skip(1);
    let db = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("gleam-search-benchmark.sqlite3"));
    let entries: usize = args
        .next()
        .map(|value| value.parse().expect("entry count must be an integer"))
        .unwrap_or(100_000);
    drop(Index::new(db.clone()).expect("create index"));
    let mut writer = Connection::open(&db).expect("open fixture writer");
    let count: usize = writer
        .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
        .unwrap();
    if count == 0 {
        let started = Instant::now();
        let transaction = writer.transaction().unwrap();
        {
            let mut insert = transaction
                .prepare(
                    "INSERT INTO files(name,name_lower,path,kind,generation) VALUES(?1,?2,?3,?4,1)",
                )
                .unwrap();
            let stems = [
                "Design system",
                "Budget",
                "Project notes",
                "Invoice",
                "Report",
                "Meeting minutes",
                "Source code",
                "日本語",
                "100% complete",
                "Archive",
            ];
            for entry in 0..entries {
                let name = format!(
                    "{} {entry:06} 2026.{}",
                    stems[entry % stems.len()],
                    if entry % 13 == 0 { "exe" } else { "txt" }
                );
                insert
                    .execute(params![
                        name,
                        name.to_lowercase(),
                        format!("C:\\Benchmark\\{name}"),
                        if entry % 13 == 0 { "app" } else { "file" }
                    ])
                    .unwrap();
            }
        }
        transaction.commit().unwrap();
        writer
            .execute_batch("PRAGMA optimize; PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
        println!(
            "Fixture: {entries} entries inserted in {:.2}s",
            started.elapsed().as_secs_f64()
        );
    }
    assert!(
        count == 0 || count == entries,
        "Existing fixture size differs; choose a new database path."
    );
    drop(writer);
    let started = Instant::now();
    let index = Index::new(db.clone()).unwrap();
    println!(
        "Index open: {:.3} ms; database: {}",
        started.elapsed().as_secs_f64() * 1000.0,
        db.display()
    );
    // The app-tier partial index is normally built by the first scan; build it for old fixtures too.
    let started = Instant::now();
    index.ensure_indexes().unwrap();
    println!("App index ensured: {:.3} ms", started.elapsed().as_secs_f64() * 1000.0);
    let cases = [
        ("des", "all"),
        ("budget", "file"),
        ("project", "all"),
        ("1", "all"),
        ("de", "all"),
        ("日本語", "all"),
        ("100%", "file"),
        ("2026", "all"),
        ("no-such-file", "all"),
        ("code", "app"),
    ];
    let mut overall = vec![];
    for (query, kind) in cases {
        index.search(query, kind).unwrap(); // Warm caches; cold-start cost is reported separately.
        let mut durations = Vec::with_capacity(30);
        let mut results = 0;
        for _ in 0..30 {
            let started = Instant::now();
            results = index.search(query, kind).unwrap().len();
            durations.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        durations.sort_by(f64::total_cmp);
        println!(
            "{query:14} {kind:4} hits={results:2} p50={:8.3} ms p95={:8.3} ms",
            durations[15], durations[28]
        );
        overall.extend(durations);
    }
    overall.sort_by(f64::total_cmp);
    println!(
        "All queries: p50={:.3} ms p95={:.3} ms ({} samples)",
        overall[overall.len() / 2],
        overall[overall.len() * 95 / 100],
        overall.len()
    );
    std::hint::black_box(index);
}
