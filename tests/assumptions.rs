mod common;
use common::*;
use std::fs;
use std::io::Write;

// ── Naked CLI ──────────────────────────────────────────────────

#[test]
fn naked_command_shows_help_with_all_commands() {
    let (out, err, ok) = run(&[]);
    assert!(!ok); // clap returns exit code 2 for "no command given"
    let combined = format!("{}{}", out, err);
    assert!(combined.contains("registry"));
    assert!(combined.contains("source"));
    assert!(combined.contains("search"));
    assert!(combined.contains("dump"));
}

#[test]
fn help_flag_shows_help() {
    let (out, _err, ok) = run(&["--help"]);
    assert!(ok);
    assert!(out.contains("registry"));
}

// ── Registry Name Assumptions ─────────────────────────────────

#[test]
fn registry_name_with_spaces_works() {
    let name = format!("my reg with spaces-{}", next_id());
    let (_out, _err, ok) = run(&["registry", "create", &name]);
    assert!(ok);
    run(&["registry", "delete", &name]);
}

#[test]
fn registry_name_with_hyphens_works() {
    let name = format!("my-reg-with-hyphens-{}", next_id());
    let (_out, _err, ok) = run(&["registry", "create", &name]);
    assert!(ok);
    run(&["registry", "delete", &name]);
}

#[test]
fn registry_name_with_underscores_works() {
    let name = format!("my_reg_with_underscores-{}", next_id());
    let (_out, _err, ok) = run(&["registry", "create", &name]);
    assert!(ok);
    run(&["registry", "delete", &name]);
}

#[test]
fn registry_show_nonexistent_fails() {
    let name = format!("no-such-reg-{}", next_id());
    let (_out, err, ok) = run(&["registry", "show", &name]);
    assert!(!ok);
    assert!(err.contains("No such file") || err.contains("cannot"));
}

#[test]
fn registry_delete_nonexistent_fails() {
    let name = format!("no-such-reg-{}", next_id());
    let (_out, _err, ok) = run(&["registry", "delete", &name]);
    assert!(!ok);
}

// ── Source URI Assumptions ────────────────────────────────────

#[test]
fn source_add_full_file_no_fragment() {
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("no-frag");
    run(&["registry", "create", &reg]);
    let (out, _err, ok) = run(&["source", "add", &reg, &format!("file://{}", file)]);
    assert!(ok, "full file URI should work: {}", out);
    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn source_add_line_range() {
    let file = write_temp("a\nb\nc\nd\ne\nf\n");
    let reg = reg_name("line-range");
    run(&["registry", "create", &reg]);
    let (out, _err, ok) = run(&[
        "source", "add", &reg,
        &format!("file://{}#L1-L5", file),
    ]);
    assert!(ok, "line range should work: {}", out);
    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn source_add_single_line() {
    let file = write_temp("a\nb\nc\n");
    let reg = reg_name("single-line");
    run(&["registry", "create", &reg]);
    let (out, _err, ok) = run(&[
        "source", "add", &reg,
        &format!("file://{}#L2", file),
    ]);
    assert!(ok, "single line should work: {}", out);
    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn source_add_same_file_twice_different_ranges() {
    let file = write_temp("a\nb\nc\nd\ne\n");
    let reg = reg_name("dup-file");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}#L1-L2", file)]);
    run(&["source", "add", &reg, &format!("file://{}#L4-L5", file)]);

    let (out, _err, ok) = run(&["source", "list", &reg]);
    assert!(ok);
    // URIs are stored relative and displayed as resolved paths — count non-header lines with content
    assert_eq!(out.lines().filter(|l| l.starts_with('0') || l.starts_with('1')).count(), 2);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn source_add_nonexistent_file_succeeds_but_search_warns() {
    let reg = reg_name("missing");
    run(&["registry", "create", &reg]);
    let (out, _err, ok) = run(&[
        "source", "add", &reg,
        "file:///no/such/file.rs",
    ]);
    assert!(ok, "adding nonexistent file should succeed: {}", out);

    let (out, _err, ok) = run(&["search", &reg, "anything"]);
    assert!(ok, "search should not crash");
    assert!(
        out.contains("not found") || out.contains("No local"),
        "should warn about missing file: {}",
        out
    );

    run(&["registry", "delete", &reg]);
}

#[test]
fn source_add_without_label_works() {
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("no-label");
    run(&["registry", "create", &reg]);
    let (out, _err, ok) = run(&["source", "add", &reg, &format!("file://{}", file)]);
    assert!(ok, "no label should work: {}", out);
    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn source_add_with_multiple_tags() {
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("multi-tag");
    run(&["registry", "create", &reg]);
    let (_out, _err, ok) = run(&[
        "source", "add", &reg,
        &format!("file://{}", file),
        "--tags", "core,string,cli,test",
    ]);
    assert!(ok);
    let (list_out, _err, _ok) = run(&["source", "list", &reg]);
    assert!(list_out.contains("core,string,cli,test"));
    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

// ── Search Assumptions ────────────────────────────────────────

#[test]
fn search_case_sensitive_by_default() {
    let file = write_temp("fn Hello() {}\n");
    let reg = reg_name("case-sens");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    // "hello" lowercase should NOT match "Hello"
    let (out, _err, ok) = run(&["search", &reg, "hello"]);
    assert!(ok);
    assert!(!out.contains("Hello"), "should be case-sensitive: {}", out);

    // "Hello" should match
    let (out, _err, ok) = run(&["search", &reg, "Hello"]);
    assert!(ok);
    assert!(out.contains("Hello"));

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn search_across_multiple_sources() {
    let file_a = write_temp("fn alpha() {}\n");
    let file_b = write_temp("fn beta() {}\n");
    let reg = reg_name("multi-src");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file_a)]);
    run(&["source", "add", &reg, &format!("file://{}", file_b)]);

    let (out, _err, ok) = run(&["search", &reg, "alpha"]);
    assert!(ok);
    assert!(out.contains("alpha"));
    assert!(!out.contains("beta"));

    let (out, _err, ok) = run(&["search", &reg, "beta"]);
    assert!(ok);
    assert!(out.contains("beta"));
    assert!(!out.contains("alpha"));

    run(&["registry", "delete", &reg]);
    cleanup(&file_a);
    cleanup(&file_b);
}

#[test]
fn search_no_matches_returns_clean() {
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("no-match");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["search", &reg, "zzz_not_found"]);
    assert!(ok, "no matches should not be an error");
    assert!(!out.contains("──"), "should not print file headers: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn search_with_special_regex_chars() {
    let file = write_temp("let x = vec![1, 2];\n");
    let reg = reg_name("regex-chars");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (_out, _err, ok) = run(&["search", &reg, "vec![1"]);
    assert!(ok);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

// ── Dump Assumptions ──────────────────────────────────────────

#[test]
fn dump_entire_file_has_all_lines() {
    let file = write_temp("line one\nline two\nline three\n");
    let reg = reg_name("full-dump");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    assert!(out.contains("line one"));
    assert!(out.contains("line two"));
    assert!(out.contains("line three"));

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn dump_line_range_excludes_other_lines() {
    let file = write_temp("alpha\nbeta\ngamma\ndelta\nepsilon\n");
    let reg = reg_name("range-dump");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}#L2-L4", file)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    // Lines 2-4 are beta, gamma, delta
    assert!(out.contains("beta"));
    assert!(out.contains("gamma"));
    assert!(out.contains("delta"));
    let content_lines: Vec<&str> = out.lines().filter(|l| l.starts_with('L')).collect();
    for line in &content_lines {
        assert!(!line.contains("alpha"), "should not include line 1: {}", line);
        assert!(!line.contains("epsilon"), "should not include line 5: {}", line);
    }

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn dump_single_line_from_middle() {
    let file = write_temp("alpha\nbeta\ngamma\ndelta\nepsilon\n");
    let reg = reg_name("mid-dump");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}#L3", file)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    // From line 3 to end: gamma, delta, epsilon
    assert!(out.contains("gamma"));
    assert!(out.contains("delta"));
    let content_lines: Vec<&str> = out.lines().filter(|l| l.starts_with('L')).collect();
    for line in &content_lines {
        assert!(!line.contains("alpha"), "should not include line 1: {}", line);
    }

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn dump_line_beyond_file_length_does_not_crash() {
    let file = write_temp("only one line\n");
    let reg = reg_name("long-range");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}#L1-L999", file)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok, "should not crash on range beyond file length");
    assert!(out.contains("only one line"));

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn dump_multiple_sources_shows_headers() {
    let file_a = write_temp("content A\n");
    let file_b = write_temp("content B\n");
    let reg = reg_name("multi-dump");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file_a)]);
    run(&["source", "add", &reg, &format!("file://{}", file_b)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    assert!(out.contains("content A"));
    assert!(out.contains("content B"));
    assert!(out.matches("// ").count() >= 2);

    run(&["registry", "delete", &reg]);
    cleanup(&file_a);
    cleanup(&file_b);
}

// ── Source Index Assumptions ───────────────────────────────────

#[test]
fn remove_first_source_reindexes_remaining() {
    let file_a = write_temp("a\n");
    let file_b = write_temp("b\n");
    let reg = reg_name("reindex");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file_a)]);
    run(&["source", "add", &reg, &format!("file://{}", file_b)]);

    run(&["source", "remove", &reg, "0"]);

    let (out, _err, ok) = run(&["source", "list", &reg]);
    assert!(ok);
    // Resolved paths no longer contain file:// — check for the temp dir path instead
    assert!(out.contains("kr-test-files"), "should still have one source");
}

#[test]
fn remove_last_source_works() {
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("last-rem");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["source", "remove", &reg, "0"]);
    assert!(ok, "removing last source should work: {}", out);

    let (list_out, _err, _ok) = run(&["source", "list", &reg]);
    assert!(list_out.contains("No sources"));

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

// ── Update Assumptions ────────────────────────────────────────

#[test]
fn update_only_label_keeps_tags() {
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("update-label");
    run(&["registry", "create", &reg]);
    run(&[
        "source", "add", &reg,
        &format!("file://{}", file),
        "--tags", "core,string",
    ]);

    run(&["source", "update", &reg, "0", "--label", "new-label"]);

    let (out, _err, ok) = run(&["source", "list", &reg]);
    assert!(ok);
    assert!(out.contains("new-label"));
    assert!(out.contains("core,string"), "tags should be preserved");

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn update_only_tags_keeps_label() {
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("update-tags");
    run(&["registry", "create", &reg]);
    run(&[
        "source", "add", &reg,
        &format!("file://{}", file),
        "--label", "original-label",
    ]);

    run(&["source", "update", &reg, "0", "--tags", "new,tags"]);

    let (out, _err, ok) = run(&["source", "list", &reg]);
    assert!(ok);
    assert!(out.contains("original-label"), "label should be preserved");
    assert!(out.contains("new,tags"));

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn update_replaces_tags_does_not_append() {
    let file = write_temp("fn test() {}\n");
    let reg = reg_name("tag-replace");
    run(&["registry", "create", &reg]);
    run(&[
        "source", "add", &reg,
        &format!("file://{}", file),
        "--tags", "vintage,retro",
    ]);

    run(&["source", "update", &reg, "0", "--tags", "fresh,modern"]);

    let (out, _err, ok) = run(&["source", "list", &reg]);
    assert!(ok);
    assert!(!out.contains("vintage"), "old tags should be gone: {}", out);
    assert!(!out.contains("retro"), "old tags should be gone: {}", out);
    assert!(out.contains("fresh,modern"));

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

// ── Edge Cases ────────────────────────────────────────────────

#[test]
fn empty_file_can_be_added_and_dumped() {
    let file = write_temp("");
    let reg = reg_name("empty-file");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (_out, _err, ok) = run(&["dump", &reg]);
    assert!(ok, "empty file should not crash dump");

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn file_with_unicode_content() {
    let file = write_temp("fn hello() -> String { \"你好世界\".to_string() }\n");
    let reg = reg_name("unicode");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    assert!(out.contains("你好世界"));

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn file_with_no_trailing_newline() {
    let tmp_dir = std::env::temp_dir().join("kr-assumptions");
    fs::create_dir_all(&tmp_dir).ok();
    let path = tmp_dir.join(format!("no-newline-{}.rs", next_id()));
    let mut f = fs::File::create(&path).expect("create file");
    f.write_all(b"fn test() {}").expect("write");
    let file = path.to_string_lossy().to_string();

    let reg = reg_name("no-newline");
    run(&["registry", "create", &reg]);
    run(&["source", "add", &reg, &format!("file://{}", file)]);

    let (out, _err, ok) = run(&["dump", &reg]);
    assert!(ok);
    assert!(out.contains("fn test()"));

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn search_in_range_source_finds_within_range_only() {
    let file = write_temp("fn alpha() {}\nfn beta() {}\nfn gamma() {}\n");
    let reg = reg_name("range-search");
    run(&["registry", "create", &reg]);
    // Only register lines 1-2 (alpha and beta, not gamma)
    run(&["source", "add", &reg, &format!("file://{}#L1-L2", file)]);

    let (out, _err, ok) = run(&["search", &reg, "alpha"]);
    assert!(ok);
    assert!(out.contains("alpha"));

    // gamma is on line 3 — outside the registered range
    let (out, _err, ok) = run(&["search", &reg, "gamma"]);
    assert!(ok);
    assert!(!out.contains("gamma"), "should not find gamma outside range: {}", out);

    run(&["registry", "delete", &reg]);
    cleanup(&file);
}

#[test]
fn registry_json_is_readable() {
    let reg = reg_name("json-check");
    run(&["registry", "create", &reg]);

    let home = dirs::home_dir().expect("home dir");
    let path = home.join(format!(".kr/{}.json", reg));
    assert!(path.exists(), "json file should exist");

    let data = fs::read_to_string(&path).expect("read json");
    let parsed: serde_json::Value = serde_json::from_str(&data).expect("valid json");
    assert_eq!(parsed["name"], reg);
    assert!(parsed["sources"].is_array());

    run(&["registry", "delete", &reg]);
}
