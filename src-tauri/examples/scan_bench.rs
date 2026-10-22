//! Reproducible indexer benchmark: a first scan, then a rescan of the unchanged tree.
//! cargo run --release --example scan_bench -- <fresh-database-path> <directory-to-scan>
use gleam_search::Index;
use std::{path::PathBuf, time::Instant};

fn scan(index: &std::sync::Arc<Index>, root: &PathBuf) -> (f64, u64) {
    let started = Instant::now();
    index.rebuild(vec![root.clone()]);
    while index.status().indexing {
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    (started.elapsed().as_secs_f64(), index.status().indexed)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let db = PathBuf::from(args.next().expect("database path"));
    let root = PathBuf::from(args.next().expect("directory to scan"));
    let index = Index::new(db).expect("create index");
    let (first, entries) = scan(&index, &root);
    println!("First scan: {entries} entries in {first:.2} s");
    for run in 1..=2 {
        let (rescan, entries) = scan(&index, &root);
        println!("Rescan {run} (unchanged tree): {entries} entries in {rescan:.2} s");
    }
}
