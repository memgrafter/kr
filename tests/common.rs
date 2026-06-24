use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn next_id() -> u64 {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn kr() -> String {
    env!("CARGO_BIN_EXE_kr").to_string()
}

/// Isolated temp folder for this test binary — created once, cleaned up on exit.
/// Each test file gets its own folder via a unique counter value at init time.
pub fn get_folder_path() -> &'static PathBuf {
    use std::sync::{OnceLock};
    static FOLDER: OnceLock<PathBuf> = OnceLock::new();
    FOLDER.get_or_init(|| {
        let tmp = std::env::temp_dir().join(format!(
            "kr-test-{}-{}",
            std::process::id(),
            next_id()
        ));
        fs::create_dir_all(&tmp).ok();
        tmp
    })
}

/// Run `kr <args...>` with KR_FOLDER set to an isolated temp directory.
/// The folder is shared across all tests in this binary and cleaned up at exit.
pub fn run(args: &[&str]) -> (String, String, bool) {
    let folder = get_folder_path();
    let out = Command::new(kr())
        .args(args)
        .env("KR_FOLDER", folder.to_string_lossy().to_string())
        .output()
        .expect("run kr");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
    )
}

/// Run `kr <args...>` with owned strings and KR_FOLDER isolation.
pub fn run_owned(args: &[String]) -> (String, String, bool) {
    let folder = get_folder_path();
    let out = Command::new(kr())
        .args(args.iter().map(|s| s.as_str()))
        .env("KR_FOLDER", folder.to_string_lossy().to_string())
        .output()
        .expect("run kr");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
    )
}

pub fn reg_name(prefix: &str) -> String {
    format!("{}-{}", prefix, next_id())
}

pub fn write_temp(content: &str) -> String {
    let tmp_dir = std::env::temp_dir().join("kr-test-files");
    fs::create_dir_all(&tmp_dir).ok();
    let path = tmp_dir.join(format!("test-{}.rs", next_id()));
    let mut f = fs::File::create(&path).expect("create temp file");
    f.write_all(content.as_bytes()).expect("write temp file");
    path.to_string_lossy().to_string()
}

pub fn cleanup(path: &str) {
    fs::remove_file(path).ok();
}
