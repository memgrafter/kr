use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;
use std::io::Write;
use std::process::Command;

fn kr() -> String {
    env!("CARGO_BIN_EXE_kr").to_string()
}

fn run(args: &[&str]) -> (String, String, bool) {
    let out = Command::new(kr()).args(args).output().expect("run kr");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
    )
}

fn setup_large_registry(name: &str, num_sources: usize) -> Vec<String> {
    let tmp_dir = std::env::temp_dir().join(format!("kr-perf-{}", name));
    fs::create_dir_all(&tmp_dir).ok();

    let mut files = Vec::new();
    for i in 0..num_sources {
        let path = tmp_dir.join(format!("source-{}.rs", i));
        let content = format!("fn func_{}() -> i32 {{ {} }}\n", i, i);
        fs::write(&path, content).ok();
        files.push(path.to_string_lossy().to_string());
    }

    run(&["registry", "create", name]);
    for file in &files {
        run(&[
            "source", "add", name,
            &format!("file://{}", file),
            "--label", &format!("source-{}", i),
        ]);
    }

    files
}

fn teardown_registry(name: &str) {
    run(&["registry", "delete", name]);
}

fn search_large_registry(c: &mut Criterion) {
    let name = "perf-search";
    let files = setup_large_registry(name, 20);

    c.bench_function("search 20 sources for 'func'", |b| {
        b.iter(|| {
            let (out, _err, ok) = run(&["search", name, "func_5"]);
            assert!(ok);
            assert!(out.contains("func_5"));
        })
    });

    teardown_registry(name);
    for f in &files {
        fs::remove_file(f).ok();
    }
    let tmp_dir = std::env::temp_dir().join(format!("kr-perf-{}", name));
    fs::remove_dir_all(&tmp_dir).ok();
}

fn dump_large_registry(c: &mut Criterion) {
    let name = "perf-dump";
    let files = setup_large_registry(name, 10);

    c.bench_function("dump 10 sources", |b| {
        b.iter(|| {
            let (out, _err, ok) = run(&["dump", name]);
            assert!(ok);
            assert!(out.contains("func_0"));
        })
    });

    teardown_registry(name);
    for f in &files {
        fs::remove_file(f).ok();
    }
    let tmp_dir = std::env::temp_dir().join(format!("kr-perf-{}", name));
    fs::remove_dir_all(&tmp_dir).ok();
}

fn glob_registration(c: &mut Criterion) {
    let tmp_dir = std::env::temp_dir().join("kr-perf-glob");
    fs::create_dir_all(&tmp_dir).ok();

    for i in 0..15 {
        let path = tmp_dir.join(format!("glob-{}.rs", i));
        fs::write(&path, format!("fn g{}() {{}}\n", i)).ok();
    }

    let name = "perf-glob";
    run(&["registry", "create", name]);

    c.bench_function("glob register 15 files", |b| {
        b.iter(|| {
            // Need fresh registry each iteration
            let iter_name = format!("perf-glob-{}", std::time::SystemTime::now().elapsed().unwrap().subsec_nanos());
            run(&["registry", "create", &iter_name]);
            let (out, _err, ok) = run(&[
                "source", "add", &iter_name,
                &format!("{}/*.rs", tmp_dir.display()),
            ]);
            assert!(ok);
            teardown_registry(&iter_name);
        })
    });

    teardown_registry(name);
    fs::remove_dir_all(&tmp_dir).ok();
}

fn tag_filter_search(c: &mut Criterion) {
    let name = "perf-tag";
    let mut files = Vec::new();
    for i in 0..12 {
        let tmp_dir = std::env::temp_dir().join("kr-perf-tag");
        fs::create_dir_all(&tmp_dir).ok();
        let path = tmp_dir.join(format!("tag-{}.rs", i));
        fs::write(&path, format!("fn t{}() {{}}\n", i)).ok();
        files.push(path.to_string_lossy().to_string());
    }

    run(&["registry", "create", name]);
    for (i, file) in files.iter().enumerate() {
        let tag = if i % 2 == 0 { "even" } else { "odd" };
        run(&[
            "source", "add", name,
            &format!("file://{}", file),
            "--tags", tag,
        ]);
    }

    c.bench_function("search with tag filter (6 of 12)", |b| {
        b.iter(|| {
            let (out, _err, ok) = run(&["search", name, "fn", "--tags", "even"]);
            assert!(ok);
        })
    });

    teardown_registry(name);
    for f in &files {
        fs::remove_file(f).ok();
    }
    let tmp_dir = std::env::temp_dir().join("kr-perf-tag");
    fs::remove_dir_all(&tmp_dir).ok();
}

fn registry_list(c: &mut Criterion) {
    // Create some registries for listing
    for i in 0..5 {
        let name = format!("perf-list-{}", i);
        if !std::path::Path::new(&format!("{}/{}.json", dirs::home_dir().unwrap().display(), name)).exists() {
            run(&["registry", "create", &name]);
        }
    }

    c.bench_function("list registries", |b| {
        b.iter(|| {
            let (out, _err, ok) = run(&["registry", "list"]);
            assert!(ok);
        })
    });

    // Cleanup
    for i in 0..5 {
        let name = format!("perf-list-{}", i);
        run(&["registry", "delete", &name]);
    }
}

criterion_group!(
    benches,
    search_large_registry,
    dump_large_registry,
    glob_registration,
    tag_filter_search,
    registry_list,
);
criterion_main!(benches);
