use std::fs;
use std::io::Write;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);
fn next_id() -> u64 {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Path to the kr binary (built by cargo test).
fn kr() -> String {
    env!("CARGO_BIN_EXE_kr").to_string()
}

/// Run `kr <args...>` and return (stdout, stderr, success).
fn run(args: &[&str]) -> (String, String, bool) {
    let out = Command::new(kr()).args(args).output().expect("run kr");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (stdout, stderr, out.status.success())
}

/// Run `kr <args...>` with owned strings and return (stdout, stderr, success).
fn run_owned(args: &[String]) -> (String, String, bool) {
    let out = Command::new(kr())
        .args(args.iter().map(|s| s.as_str()))
        .output()
        .expect("run kr");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (stdout, stderr, out.status.success())
}

/// Create a temp file with given content and return its absolute path.
fn write_temp(content: &str) -> String {
    let tmp_dir = std::env::temp_dir().join("kr-test");
    fs::create_dir_all(&tmp_dir).ok();
    let id = next_id();
    let path = tmp_dir.join(format!("test-{}.rs", id));
    let mut f = fs::File::create(&path).expect("create temp file");
    f.write_all(content.as_bytes()).expect("write temp file");
    path.to_string_lossy().to_string()
}

/// Create a unique registry name.
fn reg_name(prefix: &str) -> String {
    format!("{}-{}", prefix, next_id())
}

/// Clean up a temp file.
fn cleanup(path: &str) {
    fs::remove_file(path).ok();
}

// ── Lifecycle Tests ────────────────────────────────────────────

#[test]
fn full_lifecycle_create_add_list_search_dump_update_remove_delete() {
    let file_a = write_temp(
        "fn greet() -> String {\n    \"hello world\".to_string()\n}\n\nfn farewell() -> String {\n    \"goodbye\".to_string()\n}\n",
    );
    let file_b = write_temp(
        "use std::collections::HashMap;\n\nfn count_words(text: &str) -> HashMap<String, usize> {\n    let mut map = HashMap::new();\n    for word in text.split_whitespace() {\n        *map.entry(word.to_string()).or_insert(0) += 1;\n    }\n    map\n}\n",
    );

    let reg_name = reg_name("lifecycle");

    // 1. Create registry
    let (out, _err, ok) = run(&["registry", "create", &reg_name]);
    assert!(ok, "create failed: {}", out);
    assert!(out.contains("Created registry"));

    // 2. Add sources — full file, line range, single line
    let args: Vec<String> = vec![
        "source".into(),
        "add".into(),
        reg_name.clone(),
        format!("file://{}", file_a),
        "--label".into(),
        "module-a".into(),
        "--tags".into(),
        "core,string".into(),
    ];
    let (out, _err, ok) = run_owned(&args);
    assert!(ok, "add full file failed: {}", out);
    assert!(out.contains("Added source"));

    let args: Vec<String> = vec![
        "source".into(),
        "add".into(),
        reg_name.clone(),
        format!("file://{}#L1-L4", file_b),
        "--label".into(),
        "module-b-range".into(),
        "--tags".into(),
        "core,collections".into(),
    ];
    let (out, _err, ok) = run_owned(&args);
    assert!(ok, "add range failed: {}", out);

    let args: Vec<String> = vec![
        "source".into(),
        "add".into(),
        reg_name.clone(),
        format!("file://{}#L3", file_b),
        "--label".into(),
        "module-b-line3".into(),
    ];
    let (out, _err, ok) = run_owned(&args);
    assert!(ok, "add single line failed: {}", out);

    // 3. List sources — should show all three
    let (out, _err, ok) = run(&["source", "list", &reg_name]);
    assert!(ok, "list failed: {}", out);
    assert!(out.contains("module-a"));
    assert!(out.contains("module-b-range"));
    assert!(out.contains("module-b-line3"));
    assert!(out.contains("core,string"));
    assert!(out.contains("core,collections"));

    // 4. Show registry — should show summary
    let (out, _err, ok) = run(&["registry", "show", &reg_name]);
    assert!(ok, "show failed: {}", out);
    assert!(out.contains(&reg_name));
    assert!(out.contains("Sources:  3"));

    // 5. Search across sources
    let (out, _err, ok) = run(&["search", &reg_name, "fn greet"]);
    assert!(ok, "search failed: {}", out);
    assert!(out.contains("greet"), "search should find 'greet': {}", out);

    let (out, _err, ok) = run(&["search", &reg_name, "HashMap"]);
    assert!(ok, "search for HashMap failed: {}", out);
    assert!(out.contains("HashMap"), "search should find 'HashMap': {}", out);

    // 6. Dump full file source — should include all content
    let (out, _err, ok) = run(&["dump", &reg_name]);
    assert!(ok, "dump failed: {}", out);
    assert!(out.contains("fn greet"), "dump should contain greet fn");
    assert!(out.contains("hello world"), "dump should contain hello world");

    // 7. Update source label and tags
    let args: Vec<String> = vec![
        "source".into(),
        "update".into(),
        reg_name.clone(),
        "0".into(),
        "--label".into(),
        "module-a-updated".into(),
        "--tags".into(),
        "core,string,updated".into(),
    ];
    let (out, _err, ok) = run_owned(&args);
    assert!(ok, "update failed: {}", out);
    assert!(out.contains("Updated source"));

    let (out, _err, _ok) = run(&["source", "list", &reg_name]);
    assert!(out.contains("module-a-updated"), "label should be updated");
    assert!(out.contains("core,string,updated"), "tags should be updated");

    // 8. Remove a source
    let (out, _err, ok) = run(&["source", "remove", &reg_name, "2"]);
    assert!(ok, "remove failed: {}", out);
    assert!(
        out.contains("Removed source"),
        "should confirm removal: out={}",
        out
    );

    let (out, _err, _ok) = run(&["source", "list", &reg_name]);
    assert!(!out.contains("module-b-line3"), "removed source should not appear");
    assert!(out.contains("module-a-updated"));
    assert!(out.contains("module-b-range"));

    // 9. Delete registry
    let (out, _err, ok) = run(&["registry", "delete", &reg_name]);
    assert!(ok, "delete failed: {}", out);
    assert!(out.contains("Deleted registry"));

    // Verify it's gone
    let (_out, _err, ok) = run(&["registry", "show", &reg_name]);
    assert!(!ok, "deleted registry should not be found");

    cleanup(&file_a);
    cleanup(&file_b);
}

#[test]
fn registry_list_shows_all_registries() {
    let reg1 = reg_name("list-test-1");
    let reg2 = reg_name("list-test-2");

    run(&["registry", "create", &reg1]);
    run(&["registry", "create", &reg2]);

    let (out, _err, ok) = run(&["registry", "list"]);
    assert!(ok, "list failed: {}", out);
    assert!(out.contains(&reg1));
    assert!(out.contains(&reg2));

    run(&["registry", "delete", &reg1]);
    run(&["registry", "delete", &reg2]);
}

#[test]
fn duplicate_registry_create_fails() {
    let reg = reg_name("dup-test");
    run(&["registry", "create", &reg]);
    let (_out, err, ok) = run(&["registry", "create", &reg]);
    assert!(!ok, "duplicate create should fail");
    assert!(
        err.contains("already exists"),
        "stderr should mention already exists: {}",
        err
    );

    run(&["registry", "delete", &reg]);
}

#[test]
fn search_with_context_lines() {
    let file = write_temp(
        "fn alpha() {}\nfn beta() {}\nfn gamma() {}\nfn delta() {}\nfn epsilon() {}\n",
    );
    let reg = reg_name("ctx-test");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    // Search with 0 context — should only show the matching line
    let (out, _err, ok) = run(&["search", &reg, "gamma", "-c", "0"]);
    assert!(ok, "search with -c 0 failed: {}", out);
    assert!(out.contains("gamma"));

    // Search with 2 context — should show surrounding lines too
    let (out, _err, ok) = run(&["search", &reg, "gamma", "-c", "2"]);
    assert!(ok, "search with -c 2 failed: {}", out);
    assert!(out.contains("beta"), "should include preceding context");
    assert!(out.contains("delta"), "should include following context");

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn dump_respects_line_ranges() {
    let file = write_temp(
        "line one\nline two\nline three\nline four\nline five\n",
    );
    let reg = reg_name("range-dump-test");
    run(&["registry", "create", &reg]);
    run(&[
        "source",
        "add",
        &reg,
        &format!("file://{}#L2-L4", file),
    ]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok, "dump failed: {}", out);
    assert!(out.contains("line two"));
    assert!(out.contains("line three"));
    assert!(out.contains("line four"));
    assert!(!out.contains("line one"), "should not include line 1");
    assert!(!out.contains("line five"), "should not include line 5");

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn remove_out_of_range_fails() {
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("remove-oor-test");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (_out, err, ok) = run(&["source", "remove", &reg, "5"]);
    assert!(!ok, "removing out-of-range index should fail");
    assert!(
        err.contains("out of range"),
        "stderr should mention out of range: {}",
        err
    );

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn update_out_of_range_fails() {
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("update-oor-test");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (_out, err, ok) = run(&["source", "update", &reg, "99", "--label", "x"]);
    assert!(!ok, "updating out-of-range index should fail");
    assert!(
        err.contains("out of range"),
        "stderr should mention out of range: {}",
        err
    );

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn source_add_invalid_uri_fails() {
    let reg = reg_name("bad-uri-test");
    run(&["registry", "create", &reg]);

    // URI with unparseable line range
    let (_out, _err, ok) = run(&[
        "source",
        "add",
        &reg,
        "file:///some/path.rs#Labc-Ldef",
    ]);
    assert!(!ok, "bad URI should fail");

    run(&["registry", "delete", &reg]);
}

#[test]
fn empty_registry_search_and_dump() {
    let reg = reg_name("empty-test");
    run(&["registry", "create", &reg]);

    let (_out, _err, ok) = run(&["search", &reg, "anything"]);
    assert!(ok); // should not error, just no results

    let (_out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);

    run(&["registry", "delete", &reg]);
}

#[test]
fn file_not_found_warns_but_does_not_crash() {
    let reg = reg_name("missing-file-test");
    run(&["registry", "create", &reg]);
    run(&[
        "source",
        "add",
        &reg,
        "file:///does/not/exist.rs#L1-L10",
    ]);

    let (_out, _err, ok) = run(&["search", &reg, "anything"]);
    assert!(ok, "search should not crash on missing file");

    let (_out, _err, ok) = run(&["dump", &reg]);
    assert!(ok, "dump should not crash on missing file");

    run(&["registry", "delete", &reg]);
}
