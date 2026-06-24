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
    let tmp_dir = std::env::temp_dir().join("kr-features");
    fs::create_dir_all(&tmp_dir).ok();
    let path = tmp_dir.join(format!("test-{}.rs", next_id()));
    let mut f = fs::File::create(&path).expect("create temp file");
    f.write_all(content.as_bytes()).expect("write temp file");
    path.to_string_lossy().to_string()
}

fn cleanup(path: &str) {
    fs::remove_file(path).ok();
}

// ── kr-z1xz: Glob Registration ────────────────────────────────

#[test]
fn glob_adds_multiple_sources_at_once() {
    let tmp_dir = std::env::temp_dir().join("kr-features");
    fs::create_dir_all(&tmp_dir).ok();

    // Create multiple files
    let file_a = tmp_dir.join(format!("glob-{}-a.rs", next_id()));
    let file_b = tmp_dir.join(format!("glob-{}-b.rs", next_id()));
    let glob_prefix = file_a.file_stem().unwrap().to_string_lossy().to_string();

    fs::write(&file_a, "fn alpha() {}\n").ok();
    fs::write(&file_b, "fn beta() {}\n").ok();

    let reg = reg_name("glob-test");
    run(&["registry", "create", &reg]);

    // Use glob pattern
    let (out, _err, ok) = run(&[
        "source", "add", &reg,
        &format!("{}/*.rs", tmp_dir.display()),
        "--label", "globbed",
    ]);
    assert!(ok, "glob add should succeed: {}", out);
    assert!(out.contains("Added") && out.contains("sources from glob"));

    let (list_out, _err, _ok) = run(&["source", "list", &reg]);
    // Should have at least 2 sources from glob
    let source_count = list_out.lines().filter(|l| l.contains("file://")).count();
    assert!(source_count >= 2, "should have at least 2 globbed sources, got {}: {}", source_count, list_out);

    run(&["registry", "delete", &reg]);
    fs::remove_file(&file_a).ok();
    fs::remove_file(&file_b).ok();
}

// ── kr-11ss: Tag Filtering ────────────────────────────────────

#[test]
fn search_with_tag_filter_only_searches_matching_sources() {
    let file_core = write_temp("fn core_fn() {}\n");
    let file_util = write_temp("fn util_fn() {}\n");
    let reg = reg_name("tag-filter");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file_core), "--tags", "core"]);
    run(&["source", "add", &reg, &format!("file://{}", file_util), "--tags", "util"]);

    // Search with --tags core should only find core_fn
    let (out, _err, ok) = run(&["search", &reg, "fn", "--tags", "core"]);
    assert!(ok);
    assert!(out.contains("core_fn"), "should find core_fn: {}", out);
    assert!(!out.contains("util_fn"), "should not find util_fn: {}", out);

    // Search with --tags util should only find util_fn
    let (out, _err, ok) = run(&["search", &reg, "fn", "--tags", "util"]);
    assert!(ok);
    assert!(out.contains("util_fn"), "should find util_fn: {}", out);
    assert!(!out.contains("core_fn"), "should not find core_fn: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file_core);
    cleanup(&file_util);
}

#[test]
fn dump_with_tag_filter_only_dumps_matching_sources() {
    let file_a = write_temp("content A\n");
    let file_b = write_temp("content B\n");
    let reg = reg_name("dump-tag");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file_a), "--tags", "alpha"]);
    run(&["source", "add", &reg, &format!("file://{}", file_b), "--tags", "beta"]);

    let (out, _err, ok) = run(&["dump", &reg, "--tags", "alpha"]);
    assert!(ok);
    assert!(out.contains("content A"));
    assert!(!out.contains("content B"), "should not dump beta source: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file_a);
    cleanup(&file_b);
}

// ── kr-8m72: Clean Dump Format ────────────────────────────────

#[test]
fn dump_header_includes_range_and_label() {
    let file = write_temp("line one\nline two\nline three\n");
    let reg = reg_name("dump-format");
    run(&["registry", "create", &reg]);
    run(&[
        "source", "add", &reg,
        &format!("file://{}#L2-L3", file),
        "--label", "partial",
    ]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    // Header should have format: // path [L2-L3] — partial
    assert!(out.contains("[L2-L3]"), "header should include range: {}", out);
    assert!(out.contains("partial"), "header should include label: {}", out);
    // Content should NOT have line numbers
    assert!(!out.contains("L   2 |"), "should not have line number prefixes: {}", out);
    assert!(out.contains("line two"));
    assert!(out.contains("line three"));

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn dump_header_without_range_or_label() {
    let file = write_temp("just content\n");
    let reg = reg_name("dump-simple");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    // Header should be just: // path
    let header_line = out.lines().find(|l| l.starts_with("// ")).expect("should have header");
    assert!(!header_line.contains("["), "no range bracket: {}", header_line);
    assert!(!header_line.contains("—"), "no label dash: {}", header_line);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

// ── kr-yibl: .krrc Discovery ──────────────────────────────────

#[test]
fn krrc_sets_mode_to_all() {
    let tmp_dir = std::env::temp_dir().join(format!("kr-krrc-{}", next_id()));
    fs::create_dir_all(&tmp_dir).ok();

    // Create .krrc with mode: all
    let krrc_path = tmp_dir.join(".krrc");
    fs::write(&krrc_path, "mode: all\n").ok();

    // Create a sub-project .kr folder
    let proj_kr = tmp_dir.join(".kr");
    fs::create_dir_all(&proj_kr).ok();

    // Create a registry in the project .kr
    let status = Command::new(kr())
        .current_dir(&tmp_dir)
        .args(&["registry", "create", "krrc-test"])
        .status()
        .expect("run kr");
    assert!(status.success());

    // Discover should show mode: all
    let out = Command::new(kr())
        .current_dir(&tmp_dir)
        .args(&["discover"])
        .output()
        .expect("run kr discover");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Mode: all"), "should show mode all: {}", stdout);

    // Cleanup
    Command::new(kr())
        .current_dir(&tmp_dir)
        .args(&["registry", "delete", "krrc-test"])
        .output()
        .ok();
    fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn krrc_explicit_folders_are_included() {
    let tmp_dir = std::env::temp_dir().join(format!("kr-krrc-folders-{}", next_id()));
    let extra_kr = tmp_dir.join("extra-kr");
    fs::create_dir_all(&tmp_dir).ok();
    fs::create_dir_all(&extra_kr).ok();

    // Create .krrc with explicit folder
    let krrc_path = tmp_dir.join(".krrc");
    let extra_path = extra_kr.to_string_lossy().to_string();
    fs::write(&krrc_path, format!("mode: all\nfolders:\n  - {}\n", extra_path)).ok();

    // Discover should show the explicit folder
    let out = Command::new(kr())
        .current_dir(&tmp_dir)
        .args(&["discover"])
        .output()
        .expect("run kr discover");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("extra-kr"), "should include explicit folder: {}", stdout);

    fs::remove_dir_all(&tmp_dir).ok();
}

// ── kr-aecb: --folder Flag ────────────────────────────────────

#[test]
fn folder_flag_overrides_mode() {
    let tmp_dir = std::env::temp_dir().join(format!("kr-folder-flag-{}", next_id()));
    fs::create_dir_all(&tmp_dir).ok();

    let tmp_path = tmp_dir.to_string_lossy().to_string();
    let out = Command::new(kr())
        .args(&["--folder", &tmp_path, "discover"])
        .output()
        .expect("run kr");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Mode: override"), "should show override mode: {}", stdout);
    assert!(stdout.contains(tmp_path.as_str()), "should show specified folder: {}", stdout);

    fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn folder_flag_with_multiple_folders() {
    let dir_a = std::env::temp_dir().join(format!("kr-folder-a-{}", next_id()));
    let dir_b = std::env::temp_dir().join(format!("kr-folder-b-{}", next_id()));
    fs::create_dir_all(&dir_a).ok();
    fs::create_dir_all(&dir_b).ok();

    let path_a = dir_a.to_string_lossy().to_string();
    let path_b = dir_b.to_string_lossy().to_string();
    let folders = format!("{},{}", path_a, path_b);

    let out = Command::new(kr())
        .args(&["--folder", &folders, "discover"])
        .output()
        .expect("run kr");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("folder-a"), "should include first folder: {}", stdout);
    assert!(stdout.contains("folder-b"), "should include second folder: {}", stdout);

    fs::remove_dir_all(&dir_a).ok();
    fs::remove_dir_all(&dir_b).ok();
}

// ── kr-177o: kr discover ──────────────────────────────────────

#[test]
fn discover_shows_mode_and_folders() {
    let out = Command::new(kr())
        .args(&["discover"])
        .output()
        .expect("run kr");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Mode:"), "should show mode: {}", stdout);
    assert!(stdout.contains("Active kr folders"), "should list folders: {}", stdout);
}

// ── kr-5cbg: Import/Export ────────────────────────────────────

#[test]
fn export_produces_valid_json() {
    let reg = reg_name("export-test");
    run(&["registry", "create", &reg]);
    let file = write_temp("fn test() {}\n");
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["export", &reg]);
    assert!(ok, "export should succeed: {}", out);

    // Parse as JSON
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("should be valid JSON");
    assert_eq!(parsed["name"], reg);
    assert!(parsed["sources"].is_array());

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn export_to_file() {
    let reg = reg_name("export-file");
    run(&["registry", "create", &reg]);

    let tmp_file = std::env::temp_dir().join(format!("kr-export-{}.json", next_id()));
    let tmp_path = tmp_file.to_string_lossy().to_string();

    let (out, _err, ok) = run(&["export", &reg, "-o", &tmp_path]);
    assert!(ok, "export to file should succeed: {}", out);
    assert!(fs::metadata(&tmp_file).is_ok(), "file should exist");

    let data = fs::read_to_string(&tmp_file).expect("read file");
    let parsed: serde_json::Value = serde_json::from_str(&data).expect("valid JSON");
    assert_eq!(parsed["name"], reg);

    run(&["registry", "delete", &reg]);
    fs::remove_file(&tmp_file).ok();
}

#[test]
fn import_from_file_creates_registry() {
    // First create and export a registry
    let reg = reg_name("import-src");
    run(&["registry", "create", &reg]);
    let file = write_temp("fn imported() {}\n");
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let tmp_file = std::env::temp_dir().join(format!("kr-import-{}.json", next_id()));
    let tmp_path = tmp_file.to_string_lossy().to_string();
    run(&["export", &reg, "-o", &tmp_path]);

    // Import with a different name (need to rename in JSON first)
    let data = fs::read_to_string(&tmp_file).expect("read file");
    let mut parsed: serde_json::Value = serde_json::from_str(&data).expect("parse JSON");
    let import_name = reg_name("import-dest");
    parsed["name"] = serde_json::Value::String(import_name.clone());
    let modified = serde_json::to_string_pretty(&parsed).expect("serialize");
    fs::write(&tmp_file, &modified).expect("write modified JSON");

    let (out, _err, ok) = run(&["import", "-i", &tmp_path]);
    assert!(ok, "import should succeed: {}", out);
    assert!(out.contains(&import_name), "should mention imported registry: {}", out);

    // Verify the imported registry exists
    let (list_out, _err, _ok) = run(&["registry", "list"]);
    assert!(list_out.contains(&import_name), "imported registry should be listed");

    run(&["registry", "delete", &reg]);
    run(&["registry", "delete", &import_name]);
    cleanup(&file);
    fs::remove_file(&tmp_file).ok();
}

#[test]
fn import_rejects_duplicate_registry() {
    let reg = reg_name("import-dup");
    run(&["registry", "create", &reg]);

    let tmp_file = std::env::temp_dir().join(format!("kr-import-dup-{}.json", next_id()));
    let tmp_path = tmp_file.to_string_lossy().to_string();
    run(&["export", &reg, "-o", &tmp_path]);

    let (_out, err, ok) = run(&["import", "-i", &tmp_path]);
    assert!(!ok, "importing duplicate should fail");
    assert!(err.contains("already exists"), "should mention already exists: {}", err);

    run(&["registry", "delete", &reg]);
    fs::remove_file(&tmp_file).ok();
}
