//! Tests for URI resolution and display path formatting (kr-64zs)

mod common;
use common::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_kr"))
}

fn run(args: &[&str]) -> (String, String, bool) {
    let folder = common::get_folder_path();
    let output = Command::new(bin())
        .args(args)
        .current_dir(test_dir())
        .env("KR_FOLDER", folder.to_string_lossy().to_string())
        .output()
        .expect("run kr");
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr).to_string();
    (out, err, output.status.success())
}

fn test_dir() -> PathBuf {
    // Use ~/.cache so paths are under home → display_path shows ~/ format
    let home = dirs::home_dir().expect("home dir");
    let dir = home.join(".cache").join("kr-res-tests");
    fs::create_dir_all(&dir).ok();
    dir
}

fn reg_name(suffix: &str) -> String {
    // Include PID to avoid collisions across parallel test runs
    format!("res-{}-{}", std::process::id(), suffix)
}

fn fresh_reg(name: &str) {
    // Clean up any stale registry from prior runs, then create fresh
    let (_out, _err, _ok) = run(&["registry", "delete", name]);
    let (out, err, ok) = run(&["registry", "create", name]);
    assert!(ok, "should create registry '{}': {} {}", name, out, err);
}

fn cleanup_reg(name: &str) {
    let (_out, _err, _ok) = run(&["registry", "delete", name]);
}

// ── Resolution Tests ───────────────────────────────────────────

#[test]
fn resolve_relative_uri_in_home_kr() {
    let tmp = test_dir();
    let file = tmp.join("test.rs");
    fs::write(&file, "fn hello() {}\n").ok();

    let reg = reg_name("home-resolve");
    fresh_reg(&reg);

    let (out, _err, ok) = run(&[
        "source", "add", &reg,
        &format!("file://{}", file.display()),
    ]);
    assert!(ok, "should add source: {}", out);

    // Source list should show resolved path with ~/
    let (list_out, _err, _ok) = run(&["source", "list", &reg]);
    assert!(list_out.contains("kr-res-tests"), "path should contain test dir name: {}", list_out);

    cleanup_reg(&reg);
    fs::remove_file(&file).ok();
}

#[test]
fn resolve_uri_with_line_range_fragment() {
    let tmp = test_dir();
    let file = tmp.join("test.rs");
    fs::write(&file, "fn a() {}\nfn b() {}\nfn c() {}\n").ok();

    let reg = reg_name("frag-resolve");
    fresh_reg(&reg);

    let (out, _err, ok) = run(&[
        "source", "add", &reg,
        &format!("file://{}#L1-L2", file.display()),
    ]);
    assert!(ok, "should add source with fragment: {}", out);

    // Dump should work — fragment used for line range
    let (dump_out, _err, _ok) = run(&["dump", &reg]);
    assert!(dump_out.contains("fn a"), "should find content in range: {}", dump_out);
    assert!(!dump_out.contains("fn c"), "should not include line 3: {}", dump_out);

    cleanup_reg(&reg);
    fs::remove_file(&file).ok();
}

#[test]
fn display_path_shows_tilde_for_home_paths() {
    let tmp = test_dir();
    let file = tmp.join("test.rs");
    fs::write(&file, "fn hello() {}\n").ok();

    let reg = reg_name("tilde-display");
    fresh_reg(&reg);

    let (out, _err, ok) = run(&[
        "source", "add", &reg,
        &format!("file://{}", file.display()),
    ]);
    assert!(ok);

    // Dump should show ~/ in header
    let (dump_out, _err, _ok) = run(&["dump", &reg]);
    assert!(dump_out.trim_start().starts_with("// ~"), "dump header should start with // ~: {}", dump_out);

    cleanup_reg(&reg);
    fs::remove_file(&file).ok();
}

#[test]
fn display_path_shows_root_for_non_home_paths() {
    let file = std::env::temp_dir().join("kr-res-root-test.rs");
    fs::write(&file, "fn test() {}\n").ok();

    let reg = reg_name("root-display");
    fresh_reg(&reg);

    let (out, _err, ok) = run(&[
        "source", "add", &reg,
        &format!("file://{}", file.display()),
    ]);
    assert!(ok);

    // The resolved path won't be under home, so it shows /tmp/... not ~/...
    let (dump_out, _err, _ok) = run(&["dump", &reg]);
    let trimmed = dump_out.trim_start();
    assert!(trimmed.starts_with("//"), "should have path header: {}", dump_out);
    assert!(!trimmed.starts_with("// ~"), "should not show ~/ for non-home path: {}", dump_out);

    cleanup_reg(&reg);
    fs::remove_file(&file).ok();
}

// ── Backward Compat Tests ──────────────────────────────────────

#[test]
fn backward_compat_old_file_uri_still_works() {
    let tmp = test_dir();
    let file = tmp.join("old-style.rs");
    fs::write(&file, "fn legacy() {}\n").ok();

    let reg = reg_name("old-compat");
    fresh_reg(&reg);

    let (out, _err, ok) = run(&[
        "source", "add", &reg,
        &format!("file://{}", file.display()),
    ]);
    assert!(ok, "old-style URI should still be accepted: {}", out);

    // Search should work
    let (search_out, _err, _ok) = run(&["search", &reg, "legacy"]);
    assert!(search_out.contains("legacy"), "should search old-style source: {}", search_out);

    // Dump should work with display path
    let (dump_out, _err, _ok) = run(&["dump", &reg]);
    assert!(dump_out.contains("fn legacy"), "should dump old-style source: {}", dump_out);

    cleanup_reg(&reg);
    fs::remove_file(&file).ok();
}

#[test]
fn backward_compat_old_uri_with_fragment_works() {
    let tmp = test_dir();
    let file = tmp.join("old-frag.rs");
    fs::write(&file, "fn a() {}\nfn b() {}\nfn c() {}\n").ok();

    let reg = reg_name("old-frag-compat");
    fresh_reg(&reg);

    let (out, _err, ok) = run(&[
        "source", "add", &reg,
        &format!("file://{}#L1-L2", file.display()),
    ]);
    assert!(ok, "old-style URI with fragment should work: {}", out);

    // Search in range should work
    let (search_out, _err, _ok) = run(&["search", &reg, "fn a"]);
    assert!(search_out.contains("fn a"), "should find in range: {}", search_out);

    // fn c on line 3 should NOT be found (outside L1-L2 range)
    let (search_out2, _err, _ok2) = run(&["search", &reg, "fn c"]);
    assert!(!search_out2.contains("fn c"), "should not find outside range: {}", search_out2);

    cleanup_reg(&reg);
    fs::remove_file(&file).ok();
}

// ── Storage Format Tests ───────────────────────────────────────

#[test]
fn new_sources_stored_as_relative_uris() {
    let tmp = test_dir();
    let file = tmp.join("new-style.rs");
    fs::write(&file, "fn new() {}\n").ok();

    let reg = reg_name("new-storage");
    fresh_reg(&reg);

    run(&[
        "source", "add", &reg,
        &format!("file://{}", file.display()),
    ]);

    // Read the JSON directly to verify storage format
    let folder = common::get_folder_path();
    let json_path = folder.join(format!("{}.json", reg));
    let json_content = fs::read_to_string(&json_path).expect("read registry JSON");

    // Should NOT contain file:// prefix
    assert!(!json_content.contains("file://"), "new sources should not store file://: {}", json_content);
    // Should NOT contain absolute path with /Users/ or similar
    assert!(!json_content.contains("/Users/"), "new sources should not store absolute paths: {}", json_content);

    cleanup_reg(&reg);
    fs::remove_file(&file).ok();
}

#[test]
fn glob_sources_stored_as_relative_uris() {
    let tmp = test_dir();
    let file_a = tmp.join("glob-a.rs");
    let file_b = tmp.join("glob-b.rs");
    fs::write(&file_a, "fn a() {}\n").ok();
    fs::write(&file_b, "fn b() {}\n").ok();

    let reg = reg_name("glob-storage");
    fresh_reg(&reg);

    run(&[
        "source", "add", &reg,
        &format!("{}/*.rs", tmp.display()),
        "--label", "globbed",
    ]);

    // Read JSON to verify
    let folder = common::get_folder_path();
    let json_path = folder.join(format!("{}.json", reg));
    let json_content = fs::read_to_string(&json_path).expect("read registry JSON");

    assert!(!json_content.contains("file://"), "glob sources should not store file://: {}", json_content);

    cleanup_reg(&reg);
    fs::remove_file(&file_a).ok();
    fs::remove_file(&file_b).ok();
}

// ── Integration: Full Flow ─────────────────────────────────────

#[test]
fn full_flow_relative_storage_resolve_display() {
    let tmp = test_dir();
    let file = tmp.join("flow-test.rs");
    fs::write(&file, "fn alpha() {}\nfn beta() {}\n").ok();

    let reg = reg_name("full-flow");
    fresh_reg(&reg);

    // Add source
    let (add_out, _err, add_ok) = run(&[
        "source", "add", &reg,
        &format!("file://{}#L1-L1", file.display()),
        "--label", "alpha only",
        "--tags", "test",
    ]);
    assert!(add_ok, "should add: {}", add_out);

    // List shows resolved path
    let (list_out, _err, _ok) = run(&["source", "list", &reg]);
    assert!(list_out.contains("alpha only"), "should show label: {}", list_out);
    assert!(!list_out.contains("file://"), "should not show file:// in list: {}", list_out);

    // Search works with resolved path
    let (search_out, _err, _ok) = run(&["search", &reg, "alpha"]);
    assert!(search_out.contains("alpha"), "should find alpha: {}", search_out);

    // Dump shows ~/ in header
    let (dump_out, _err, _ok) = run(&["dump", &reg]);
    assert!(dump_out.trim_start().starts_with("// ~"), "dump should show ~/ header: {}", dump_out);
    assert!(dump_out.contains("fn alpha"), "should contain content: {}", dump_out);

    // Tag filter works
    let (tag_search, _err, _ok) = run(&["search", &reg, "alpha", "--tags", "test"]);
    assert!(tag_search.contains("alpha"), "tag filter should work: {}", tag_search);

    cleanup_reg(&reg);
    fs::remove_file(&file).ok();
}
