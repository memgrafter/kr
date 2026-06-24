mod common;
use common::*;
use std::fs;

// ── Name Validation: reject path traversal characters ──────────

#[test]
fn registry_delete_rejects_double_dot_in_name() {
    // A name containing ".." should be rejected outright — not even attempted
    let (_out, err, ok) = run(&["registry", "delete", "../etc/passwd"]);
    assert!(!ok, "delete with .. in name must fail");
    assert!(
        err.to_lowercase().contains("invalid") || err.to_lowercase().contains("rejected"),
        "error should explain the name is invalid (not just a filesystem error), got: {}",
        err
    );
}

#[test]
fn registry_delete_rejects_forward_slash_in_name() {
    let (_out, err, ok) = run(&["registry", "delete", "foo/bar"]);
    assert!(!ok, "delete with / in name must fail");
    assert!(
        err.to_lowercase().contains("invalid") || err.to_lowercase().contains("rejected"),
        "error should explain the name is invalid (not just a filesystem error), got: {}",
        err
    );
}

#[test]
fn registry_delete_rejects_backslash_in_name() {
    let (_out, err, ok) = run(&["registry", "delete", r"foo\bar"]);
    assert!(!ok, "delete with \\ in name must fail");
    assert!(
        err.to_lowercase().contains("invalid") || err.to_lowercase().contains("rejected"),
        "error should explain the name is invalid (not just a filesystem error), got: {}",
        err
    );
}

// ── Existence + type check ────────────────────────────────────

#[test]
fn registry_delete_nonexistent_gives_clean_error() {
    let name = reg_name("no-such-reg");
    let (_out, err, ok) = run(&["registry", "delete", &name]);
    assert!(!ok, "deleting nonexistent registry must fail");
    assert!(
        err.to_lowercase().contains("not found") || err.to_lowercase().contains("does not exist"),
        "error should say registry not found, got: {}",
        err
    );
}

#[test]
fn registry_delete_refuses_directory_named_json() {
    let name = reg_name("dir-test");
    let folder = get_folder_path();
    let dir_path = folder.join(format!("{}.json", name));

    fs::create_dir(&dir_path).ok();

    let (_out, err, ok) = run(&["registry", "delete", &name]);
    assert!(!ok, "deleting a directory named {name}.json must fail");
    assert!(
        err.to_lowercase().contains("not a file") || err.to_lowercase().contains("directory") || err.to_lowercase().contains("refused"),
        "error should explain it's not a regular file, got: {}",
        err
    );

    assert!(dir_path.is_dir(), "directory must survive the failed delete attempt");
}

// ── Path confinement ──────────────────────────────────────────

#[test]
fn registry_delete_does_not_escape_kr_folder() {
    let folder = get_folder_path();
    let decoy_path = folder.join("decoy.json");
    fs::write(&decoy_path, "{\"decoy\": true}").expect("create decoy");

    let (_out, _err, _ok) = run(&["registry", "delete", "../../tmp/kr-safety-decoy/decoy"]);
    assert!(decoy_path.exists(), "decoy file must survive — delete must not escape .kr/");
}

// ── Content verification ──────────────────────────────────────

#[test]
fn registry_delete_verifies_content_matches_name() {
    let name = reg_name("content-test");
    let folder = get_folder_path();
    let file_path = folder.join(format!("{}.json", name));

    let fake_json = serde_json::json!({
        "name": "totally-different-registry",
        "created": "2024-01-01T00:00:00Z",
        "description": null,
        "sources": []
    });
    fs::write(&file_path, serde_json::to_string_pretty(&fake_json).unwrap()).ok();

    let (_out, err, ok) = run(&["registry", "delete", &name]);
    assert!(!ok, "deleting a file with mismatched content must fail");
    assert!(
        err.to_lowercase().contains("mismatch") || err.to_lowercase().contains("does not match") || err.to_lowercase().contains("not a registry"),
        "error should explain content mismatch, got: {}",
        err
    );

    assert!(file_path.exists(), "file with wrong content must survive the failed delete");
}

#[test]
fn registry_delete_refuses_corrupt_json() {
    let name = reg_name("corrupt-test");
    let folder = get_folder_path();
    let file_path = folder.join(format!("{}.json", name));

    fs::write(&file_path, "this is not json at all").ok();

    let (_out, err, ok) = run(&["registry", "delete", &name]);
    assert!(!ok, "deleting a corrupt JSON file must fail");
    assert!(
        err.to_lowercase().contains("invalid") || err.to_lowercase().contains("corrupt") || err.to_lowercase().contains("parse"),
        "error should explain the file is not valid, got: {}",
        err
    );

    assert!(file_path.exists(), "corrupt file must survive the failed delete");
}

// ── Source remove guards ──────────────────────────────────────

#[test]
fn source_remove_does_not_delete_source_file() {
    let file = write_temp("fn important_data() {}\n");
    let name = reg_name("src-remove-test");

    run(&["registry", "create", &name]);
    let (out, _err, ok) = run(&["source", "add", &name, &format!("file://{}", file)]);
    assert!(ok, "add source failed: {}", out);

    let (out, _err, ok) = run(&["source", "remove", &name, "0"]);
    assert!(ok, "remove source failed: {}", out);

    assert!(
        std::path::Path::new(&file).exists(),
        "source file must survive source remove"
    );

    run(&["registry", "delete", &name]);
    cleanup(&file);
}

// ── Test isolation ────────────────────────────────────────────

#[test]
fn test_registries_do_not_pollute_real_kr_folder() {
    let name = reg_name("isolation-check");

    run(&["registry", "create", &name]);

    // Check that the registry does NOT exist in the real home .kr folder
    let home_kr = dirs::home_dir()
        .expect("no home dir")
        .join(".kr")
        .join(format!("{}.json", name));

    assert!(
        !home_kr.exists(),
        "test registry '{}' must NOT exist in real ~/.kr/ — test isolation failure",
        name
    );

    run(&["registry", "delete", &name]);
}
