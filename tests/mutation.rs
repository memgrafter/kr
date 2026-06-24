//! Mutation-style tests: verify specific invariants that would fail if key logic was changed.
//! Each test checks a precise behavior — if the code was mutated (e.g., condition flipped),
//! these tests would catch it.

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

static mut COUNTER: u64 = 0;
fn next_id() -> u64 {
    unsafe { COUNTER += 1; COUNTER }
}

fn reg_name(prefix: &str) -> String {
    format!("{}-{}", prefix, next_id())
}

fn write_temp(content: &str) -> String {
    let tmp_dir = std::env::temp_dir().join("kr-mutation");
    fs::create_dir_all(&tmp_dir).ok();
    let path = tmp_dir.join(format!("mut-{}.rs", next_id()));
    let mut f = fs::File::create(&path).expect("create temp file");
    f.write_all(content.as_bytes()).expect("write temp file");
    path.to_string_lossy().to_string()
}

fn cleanup(path: &str) {
    fs::remove_file(path).ok();
}

// ── Search Invariants ─────────────────────────────────────────

#[test]
fn search_returns_no_results_when_query_absent() {
    // If search logic was mutated to always return results, this fails
    let file = write_temp("fn hello() {}\n");
    let reg = reg_name("search-absent");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["search", &reg, "zzz_not_here"]);
    assert!(ok);
    assert!(!out.contains("hello"), "should not return results for absent query: {}", out);
    assert!(!out.contains("──"), "should not print file headers with no matches: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn search_respects_line_range_excludes_out_of_range() {
    // If range filtering was mutated (e.g., always full file), this fails
    let file = write_temp("fn alpha() {}\nfn beta() {}\nfn gamma() {}\n");
    let reg = reg_name("search-range");
    run(&["registry", "create", &reg]);
    // Only register lines 1-2 (alpha and beta)
    run(&["source", "add", &reg, &format!("file://{}#L1-L2", file)]);

    let (out, _err, ok) = run(&["search", &reg, "gamma"]);
    assert!(ok);
    assert!(!out.contains("gamma"), "should not find gamma outside range: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn search_case_sensitivity_not_mutated() {
    // If case-insensitive flag was accidentally added, this fails
    let file = write_temp("fn Hello() {}\n");
    let reg = reg_name("search-case");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["search", &reg, "hello"]);
    assert!(ok);
    assert!(!out.contains("Hello"), "should be case-sensitive: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

// ── Dump Invariants ───────────────────────────────────────────

#[test]
fn dump_range_excludes_lines_before_start() {
    // If range start was mutated (e.g., always 0), this fails
    let file = write_temp("line1\nline2\nline3\n");
    let reg = reg_name("dump-before");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}#L2-L3", file)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    let content_lines: Vec<&str> = out.lines().filter(|l| l.starts_with("line")).collect();
    assert!(!content_lines.iter().any(|l| l.starts_with("line1")), "should exclude line 1: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn dump_range_excludes_lines_after_end() {
    // If range end was mutated (e.g., always usize::MAX), this fails
    let file = write_temp("line1\nline2\nline3\n");
    let reg = reg_name("dump-after");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}#L1-L2", file)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    let content_lines: Vec<&str> = out.lines().filter(|l| l.starts_with("line")).collect();
    assert!(!content_lines.iter().any(|l| l.starts_with("line3")), "should exclude line 3: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn dump_no_line_numbers_in_content() {
    // If line number formatting was mutated back, this fails
    let file = write_temp("content\n");
    let reg = reg_name("dump-linenum");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    // Content lines should not have "L   1 |" prefix
    for line in out.lines() {
        if line.contains("content") {
            assert!(!line.starts_with("L"), "should not have line number prefix: {}", line);
        }
    }

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

// ── Tag Filter Invariants ─────────────────────────────────────

#[test]
fn tag_filter_excludes_non_matching_tags() {
    // If tag filter was mutated to include all sources, this fails
    let file_a = write_temp("core content\n");
    let file_b = write_temp("util content\n");
    let reg = reg_name("tag-exclude");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file_a), "--tags", "core"]);
    run(&["source", "add", &reg, &format!("file://{}", file_b), "--tags", "util"]);

    let (out, _err, ok) = run(&["search", &reg, "content", "--tags", "core"]);
    assert!(ok);
    assert!(!out.contains("util content"), "should exclude non-matching tag: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file_a);
    cleanup(&file_b);
}

#[test]
fn tag_filter_empty_tags_returns_all() {
    // If empty tags was mutated to return nothing, this fails
    let file = write_temp("some content\n");
    let reg = reg_name("tag-empty");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["search", &reg, "content"]);
    assert!(ok);
    assert!(out.contains("some content"), "no tag filter should return all: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

// ── Registry Invariants ───────────────────────────────────────

#[test]
fn delete_makes_registry_unaccessible() {
    // If delete was mutated to be a no-op, this fails
    let reg = reg_name("delete-check");
    run(&["registry", "create", &reg]);

    let (_out, _err, ok) = run(&["registry", "show", &reg]);
    assert!(ok);

    run(&["registry", "delete", &reg]);

    let (_out, err, ok) = run(&["registry", "show", &reg]);
    assert!(!ok, "deleted registry should be inaccessible");
    assert!(err.contains("No such file") || err.contains("cannot"), "should error on deleted registry: {}", err);
}

#[test]
fn duplicate_create_is_rejected() {
    // If duplicate check was mutated away, this fails
    let reg = reg_name("dup-check");
    run(&["registry", "create", &reg]);

    let (_out, err, ok) = run(&["registry", "create", &reg]);
    assert!(!ok, "duplicate create should fail");
    assert!(err.contains("already exists"), "should mention already exists: {}", err);

    run(&["registry", "delete", &reg]);
}

#[test]
fn source_remove_reindexes_remaining() {
    // If reindexing was mutated away, this fails
    let file_a = write_temp("a\n");
    let file_b = write_temp("b\n");
    let reg = reg_name("reindex-check");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file_a)]);
    run(&["source", "add", &reg, &format!("file://{}", file_b)]);

    // Remove source 0 — source 1 should become 0
    run(&["source", "remove", &reg, "0"]);

    // Index 1 should now be out of range
    let (_out, err, ok) = run(&["source", "remove", &reg, "1"]);
    assert!(!ok, "index 1 should be out of range after reindex");
    assert!(err.contains("out of range"), "should mention out of range: {}", err);

    run(&["registry", "delete", &reg]);
    cleanup(&file_a);
    cleanup(&file_b);
}

// ── Update Invariants ─────────────────────────────────────────

#[test]
fn update_only_label_preserves_tags() {
    // If tag preservation was mutated (e.g., always cleared), this fails
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("update-preserve");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file), "--tags", "core,string"]);

    run(&["source", "update", &reg, "0", "--label", "new-label"]);

    let (out, _err, ok) = run(&["source", "list", &reg]);
    assert!(ok);
    assert!(out.contains("core,string"), "tags should be preserved: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn update_tags_replaces_not_appends() {
    // If tag replacement was mutated to append, this fails
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("update-replace");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file), "--tags", "old"]);

    run(&["source", "update", &reg, "0", "--tags", "new"]);

    let (out, _err, ok) = run(&["source", "list", &reg]);
    assert!(ok);
    assert!(!out.contains("old,"), "old tags should be replaced: {}", out);
    assert!(out.contains("new"), "new tags should be present");

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

// ── Export/Import Invariants ───────────────────────────────────

#[test]
fn export_contains_all_sources() {
    // If source count was mutated (e.g., truncated), this fails
    let reg = reg_name("export-count");
    run(&["registry", "create", &reg]);
    let file_a = write_temp("a\n");
    let file_b = write_temp("b\n");
    run(&["source", "add", &reg, &format!("file://{}", file_a)]);
    run(&["source", "add", &reg, &format!("file://{}", file_b)]);

    let (out, _err, ok) = run(&["export", &reg]);
    assert!(ok);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(parsed["sources"].as_array().unwrap().len(), 2, "should have all sources");

    run(&["registry", "delete", &reg]);
    cleanup(&file_a);
    cleanup(&file_b);
}

#[test]
fn import_preserves_source_order() {
    // If source order was mutated (e.g., reversed), this fails
    let reg_src = reg_name("import-order-src");
    let reg_dst = reg_name("import-order-dst");
    // Clean up any leftover from previous runs
    run(&["registry", "delete", &reg_src]); // ignore error if doesn't exist
    run(&["registry", "delete", &reg_dst]);
    run(&["registry", "create", &reg_src]);
    let file_a = write_temp("first\n");
    let file_b = write_temp("second\n");
    run(&["source", "add", &reg_src, &format!("file://{}", file_a)]);
    run(&["source", "add", &reg_src, &format!("file://{}", file_b)]);

    // Build JSON directly instead of using export (avoids multi-folder lookup issues)
    let data = format!(
        r#"{{"name":"{}","created":"2024-01-01T00:00:00+00:00","sources":[{{"uri":"file://{}","label":null,"tags":[],"added":"2024-01-01T00:00:00+00:00"}},{{"uri":"file://{}","label":null,"tags":[],"added":"2024-01-01T00:00:00+00:00"}}]}}"#,
        reg_src, file_a, file_b
    );
    let tmp_file = std::env::temp_dir().join(format!("kr-import-order-{}.json", next_id()));
    fs::write(&tmp_file, &data).expect("write json");
    let tmp_path = tmp_file.to_string_lossy().to_string();

    // Modify name for import
    let modified = data.replace(&reg_src, &reg_dst);
    fs::write(&tmp_file, &modified).expect("write modified json");

    run(&["import", "-i", &tmp_path]);

    // Verify source order by reading the imported file directly
    let home = dirs::home_dir().expect("home");
    let imported_path = home.join(format!(".kr/{}.json", reg_dst));
    let imported_data = fs::read_to_string(&imported_path).expect("read imported");
    let parsed: serde_json::Value = serde_json::from_str(&imported_data).expect("valid JSON");
    let sources = parsed["sources"].as_array().expect("sources array");
    assert_eq!(sources.len(), 2, "should have 2 sources");
    let first_uri = sources[0]["uri"].as_str().expect("uri string");
    let second_uri = sources[1]["uri"].as_str().expect("uri string");
    assert!(first_uri.contains(&file_a), "first source should be file_a: {}", first_uri);
    assert!(second_uri.contains(&file_b), "second source should be file_b: {}", second_uri);

    run(&["registry", "delete", &reg_src]);
    run(&["registry", "delete", &reg_dst]);
    cleanup(&file_a);
    cleanup(&file_b);
    fs::remove_file(&tmp_file).ok();
}

// ── Mode Invariants ───────────────────────────────────────────

#[test]
fn single_mode_only_uses_current_folder() {
    // If mode was mutated to always be "all", this fails
    let out = Command::new(kr())
        .args(&["discover"])
        .output()
        .expect("run kr discover");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Mode: single"), "default mode should be single: {}", stdout);
}

#[test]
fn folder_flag_overrides_discovery() {
    // If --folder was mutated to be ignored, this fails
    let tmp_dir = std::env::temp_dir().join(format!("kr-mode-override-{}", next_id()));
    fs::create_dir_all(&tmp_dir).ok();
    let tmp_path = tmp_dir.to_string_lossy().to_string();

    let out = Command::new(kr())
        .args(&["--folder", &tmp_path, "discover"])
        .output()
        .expect("run kr");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Mode: override"), "should show override mode: {}", stdout);
    assert!(!stdout.contains("Mode: single"), "should not be single mode: {}", stdout);

    fs::remove_dir_all(&tmp_dir).ok();
}
