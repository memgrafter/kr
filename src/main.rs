use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use glob::glob;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// ── Domain Models ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Source {
    uri: String,
    label: Option<String>,
    tags: Vec<String>,
    added: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Registry {
    name: String,
    created: String,
    description: Option<String>,
    sources: Vec<Source>,
    #[serde(skip)]
    kr_folder: PathBuf,
}

// ── .krrc Config ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KrrcConfig {
    mode: Option<String>,
    folders: Option<Vec<String>>,
}

fn default_mode() -> String {
    "single".to_string()
}

// ── Discovery ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct DiscoveryResult {
    mode: String,
    current_folder: PathBuf,
    all_folders: Vec<PathBuf>,
    explicit_folders: Vec<PathBuf>,
}

fn find_krrc(path: &Path) -> Option<KrrcConfig> {
    let krrc_path = path.join(".krrc");
    if krrc_path.exists() {
        let data = fs::read_to_string(&krrc_path).ok()?;
        serde_yaml::from_str(&data).ok()
    } else {
        None
    }
}

fn discover_kr_folders(folder_override: Option<&[String]>) -> DiscoveryResult {
    // If --folder is specified, use only those folders
    if let Some(folders) = folder_override {
        let paths: Vec<PathBuf> = folders.iter().map(|f| {
            let p = shellexpand::full(f).expect("expand path").into_owned();
            PathBuf::from(p)
        }).collect();
        return DiscoveryResult {
            mode: "override".to_string(),
            current_folder: paths[0].clone(),
            all_folders: paths,
            explicit_folders: vec![],
        };
    }

    let home = dirs::home_dir().expect("cannot find home directory");
    let cwd = std::env::current_dir().expect("cannot get cwd");

    // Walk from pwd up to home, collecting .krrc configs and .kr folders
    let mut all_folders: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut current_folder: Option<PathBuf> = None;
    let mut mode = default_mode();
    let mut explicit_folders: Vec<PathBuf> = Vec::new();

    let mut current = cwd.clone();
    loop {
        // Check for .krrc
        if let Some(config) = find_krrc(&current) {
            if let Some(m) = config.mode {
                mode = m;
            }
            if let Some(folders) = config.folders {
                for f in folders {
                    let p = shellexpand::full(&f).expect("expand path").into_owned();
                    let path = PathBuf::from(p);
                    explicit_folders.push(path.clone());
                    let key = path.to_string_lossy().to_string();
                    if seen.insert(key) {
                        all_folders.push(path);
                    }
                }
            }
        }

        // Check for .kr folder at this level
        let kr_folder = current.join(".kr");
        if kr_folder.exists() && kr_folder.is_dir() {
            let key = kr_folder.to_string_lossy().to_string();
            if seen.insert(key) {
                all_folders.push(kr_folder.clone());
                // Only set current_folder on first find (closest to cwd)
                if current_folder.is_none() {
                    current_folder = Some(kr_folder);
                }
            }
        }

        // Move up or stop
        if current == *home || !current.pop() {
            break;
        }
    }

    // Add home .kr as fallback
    let home_kr = home.join(".kr");
    let key = home_kr.to_string_lossy().to_string();
    if seen.insert(key) {
        all_folders.push(home_kr);
    }

    // In single mode, only use current folder (closest .kr to cwd, or home fallback)
    let current_folder = current_folder.unwrap_or_else(|| home.join(".kr"));
    let effective_folders = if mode == "single" {
        vec![current_folder.clone()]
    } else {
        all_folders.clone()
    };

    DiscoveryResult {
        mode,
        current_folder,
        all_folders: effective_folders,
        explicit_folders,
    }
}

// ── Storage (multi-folder aware) ───────────────────────────────

fn registry_dir() -> PathBuf {
    let discovery = discover_kr_folders(None);
    discovery.current_folder.clone()
}

fn registry_dirs() -> Vec<PathBuf> {
    discover_kr_folders(None).all_folders.clone()
}

fn registry_path(name: &str) -> Option<PathBuf> {
    for dir in registry_dirs() {
        let path = dir.join(format!("{name}.json"));
        if path.exists() {
            return Some(path);
        }
    }
    // Default to first folder
    Some(registry_dir().join(format!("{name}.json")))
}

fn save_registry(registry: &Registry) -> Result<()> {
    let path = registry_dir().join(format!("{}.json", registry.name));
    fs::create_dir_all(&registry_dir()).ok();
    let data = serde_json::to_string_pretty(registry).context("serialize registry")?;
    fs::write(&path, data).with_context(|| format!("write to {}", path.display()))
}

/// Load a registry and set its kr_folder from the file location.
/// kr_folder is the .kr directory that holds this registry file.
fn load_registry(name: &str) -> Result<Registry> {
    let path = registry_path(name)
        .ok_or_else(|| anyhow::anyhow!("registry '{}' not found in any kr folder", name))?;
    let data = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut reg: Registry = serde_json::from_str(&data).context("parse registry JSON")?;
    // Set kr_folder to the .kr directory holding this file
    if let Some(kr_dir) = path.parent() {
        reg.kr_folder = kr_dir.to_path_buf();
    } else {
        reg.kr_folder = registry_dir();
    }
    Ok(reg)
}

fn list_registries() -> Result<Vec<Registry>> {
    let mut registries = Vec::new();
    for dir in registry_dirs() {
        if !dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&dir).context(format!("read {}", dir.display()))? {
            let entry = entry.context("read directory entry")?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                // Load without kr_folder for listing — just need metadata
                let data = fs::read_to_string(&path).ok();
                if let Some(data) = data {
                    let mut reg: Registry = serde_json::from_str(&data).ok().unwrap_or_else(|| {
                        Registry {
                            name: name.to_string(),
                            created: "?".to_string(),
                            description: None,
                            sources: Vec::new(),
                            kr_folder: PathBuf::new(),
                        }
                    });
                    reg.kr_folder = dir.clone();
                    registries.push(reg);
                }
            }
        }
    }
    // Deduplicate by name
    let mut seen = HashSet::new();
    registries.retain(|r| seen.insert(r.name.clone()));
    Ok(registries)
}

// ── URI Parsing ────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ParsedUri {
    scheme: String,
    path: Option<String>,
    line_start: Option<usize>,
    line_end: Option<usize>,
}

fn parse_uri(uri: &str) -> Result<ParsedUri> {
    let (base, fragment) = match uri.split_once('#') {
        Some((b, f)) => (b, Some(f)),
        None => (uri, None),
    };

    let (scheme, path_str) = match base.split_once("://") {
        Some((s, p)) => (s.to_string(), Some(p.to_string())),
        None => ("file".to_string(), None),
    };

    let mut line_start: Option<usize> = None;
    let mut line_end: Option<usize> = None;

    if let Some(frag) = fragment {
        let frag = frag.trim();
        if let Some(range) = frag.strip_prefix("L") {
            if let Some((s, rest)) = range.split_once('-') {
                line_start = Some(s.parse().context("parse start line")?);
                line_end = Some(rest.trim_start_matches('L').parse().context("parse end line")?);
            } else {
                line_start = Some(range.parse().context("parse single line")?);
            }
        }
    }

    Ok(ParsedUri {
        scheme,
        path: path_str,
        line_start,
        line_end,
    })
}

// ── Tag Filtering ──────────────────────────────────────────────

fn filter_sources_by_tags<'a>(sources: &'a [Source], tags: &[String]) -> Vec<&'a Source> {
    if tags.is_empty() {
        sources.iter().collect()
    } else {
        sources
            .iter()
            .filter(|s| s.tags.iter().any(|t| tags.contains(t)))
            .collect()
    }
}

// ── Path Resolution ────────────────────────────────────────────

/// Resolve a stored URI to an absolute file path.
/// Stored URIs are relative to the parent of the .kr folder.
/// Backward compat: absolute URIs (starting with /) are returned as-is.
fn resolve_uri(kr_folder: &Path, uri: &str) -> PathBuf {
    // Strip fragment (line range) first
    let without_frag = uri.split('#').next().unwrap_or(uri);

    // Strip file:// prefix if present
    let clean = without_frag.strip_prefix("file://").unwrap_or(without_frag);

    // If already absolute, return as-is (backward compat with old registries)
    if clean.starts_with('/') {
        return PathBuf::from(clean);
    }

    // Expand ~ in the URI itself before joining
    let expanded = expand_tilde(clean);

    // Join with the parent of the .kr folder
    let base = kr_folder.parent().unwrap_or_else(|| Path::new("/"));
    base.join(expanded)
}

/// Display an absolute path as ~/... if under home, /... otherwise.
fn display_path(abs: &Path) -> String {
    let home = dirs::home_dir().expect("cannot find home directory");
    if let Ok(stripped) = abs.strip_prefix(&home) {
        format!("~/{}", stripped.display())
    } else {
        format!("{}", abs.display())
    }
}

/// Expand ~ at the start of a path string.
fn expand_tilde(path_str: &str) -> String {
    if let Some(remainder) = path_str.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(remainder).to_string_lossy().to_string();
        }
    }
    path_str.to_string()
}

/// Convert an absolute path to a relative URI suitable for storage.
/// If the path is under the .kr parent, store as relative.
/// If under home but not under .kr parent, store as ~/relative.
/// Otherwise fall back to absolute (shouldn't happen in normal use).
fn to_stored_uri(kr_folder: &Path, abs_path: &Path) -> String {
    let base = kr_folder.parent().unwrap_or_else(|| Path::new("/"));

    // Try relative to .kr parent first
    if let Ok(rel) = abs_path.strip_prefix(base) {
        return rel.to_string_lossy().to_string();
    }

    // Try under home
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = abs_path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }

    // Fallback: absolute (shouldn't happen in normal use)
    abs_path.to_string_lossy().to_string()
}

// ── Range Helpers ──────────────────────────────────────────────

fn range_label(start: Option<usize>, end: Option<usize>) -> String {
    match (start, end) {
        (Some(s), Some(e)) => format!("L{}-L{}", s, e),
        (Some(s), None) => format!("L{}+", s),
        _ => String::new(),
    }
}

fn extract_lines(content: &str, start: Option<usize>, end: Option<usize>) -> Vec<&str> {
    let lines: Vec<&str> = content.lines().collect();
    match (start, end) {
        (Some(s), Some(e)) => {
            let si = (s - 1).min(lines.len());
            let ei = e.min(lines.len());
            lines[si..ei].to_vec()
        }
        (Some(s), None) => {
            let si = (s - 1).min(lines.len());
            lines[si..].to_vec()
        }
        _ => lines,
    }
}

// ── Search ─────────────────────────────────────────────────────

fn search_registry(registry: &Registry, query: &str, context: usize, tags: &[String]) -> Result<()> {
    let filtered = filter_sources_by_tags(&registry.sources, tags);
    let mut file_targets: Vec<(PathBuf, Option<usize>, Option<usize>)> = Vec::new();

    for source in filtered {
        let parsed = parse_uri(&source.uri).context(format!("parse URI {}", source.uri))?;
        let path = resolve_uri(&registry.kr_folder, &source.uri);
        if path.exists() {
            file_targets.push((path.clone(), parsed.line_start, parsed.line_end));
        } else {
            eprintln!("⚠  File not found: {}", display_path(&path));
        }
    }

    if file_targets.is_empty() {
        println!("No local file sources to search.");
        return Ok(());
    }

    for (path, start, end) in &file_targets {
        let path_str = path.to_string_lossy().to_string();
        let display = display_path(path);

        match (start, end) {
            (Some(s), Some(e)) => {
                let mut cmd = std::process::Command::new("sed");
                cmd.arg("-n").arg(format!("{},{}p", s, e)).arg(path_str.clone());
                let sed_out = cmd.output().context("run sed")?;
                let lines = String::from_utf8_lossy(&sed_out.stdout);
                let mut rg = std::process::Command::new("rg");
                rg.arg("--context")
                    .arg(context.to_string())
                    .arg(query)
                    .stdin(std::process::Stdio::piped());
                let mut child = rg.spawn().expect("spawn rg");
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    stdin.write_all(lines.as_bytes()).ok();
                }
                let result = child.wait_with_output().context("wait for rg")?;
                let stdout = String::from_utf8_lossy(&result.stdout);
                if !stdout.is_empty() {
                    println!("\n── {} (L{}-L{}) ──", display, s, e);
                    print!("{}", stdout);
                }
                continue;
            }
            (Some(s), None) => {
                let mut cmd = std::process::Command::new("sed");
                cmd.arg("-n");
                cmd.arg(format!("{},$p", s));
                cmd.arg(path_str.clone());
                let sed_out = cmd.output().context("run sed")?;
                let lines = String::from_utf8_lossy(&sed_out.stdout);
                let mut rg = std::process::Command::new("rg");
                rg.arg("--context")
                    .arg(context.to_string())
                    .arg(query)
                    .stdin(std::process::Stdio::piped());
                let mut child = rg.spawn().expect("spawn rg");
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    stdin.write_all(lines.as_bytes()).ok();
                }
                let result = child.wait_with_output().context("wait for rg")?;
                let stdout = String::from_utf8_lossy(&result.stdout);
                if !stdout.is_empty() {
                    println!("\n── {} (L{}+) ──", display, s);
                    print!("{}", stdout);
                }
                continue;
            }
            _ => {}
        }

        let mut cmd = std::process::Command::new("rg");
        cmd.arg("--context")
            .arg(context.to_string())
            .arg(query)
            .arg(path.to_string_lossy().to_string());

        let output = cmd.output().context("run rg")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            println!("\n── {} ──", display);
            print!("{}", stdout);
        }
    }

    Ok(())
}

// ── Dump ───────────────────────────────────────────────────────

fn dump_registry(registry: &Registry, tags: &[String]) -> Result<()> {
    let filtered = filter_sources_by_tags(&registry.sources, tags);

    for source in filtered {
        let parsed = parse_uri(&source.uri).context(format!("parse URI {}", source.uri))?;
        let path = resolve_uri(&registry.kr_folder, &source.uri);
        if !path.exists() {
            eprintln!("⚠  File not found: {}", display_path(&path));
            continue;
        }

        let display = display_path(&path);
        let range = range_label(parsed.line_start, parsed.line_end);
        if let Some(ref label) = source.label {
            if !range.is_empty() {
                println!("\n// {} [{}] — {}", display, range, label);
            } else {
                println!("\n// {} — {}", display, label);
            }
        } else if !range.is_empty() {
            println!("\n// {} [{}]", display, range);
        } else {
            println!("\n// {}", display);
        }

        let content = fs::read_to_string(&path).with_context(|| format!("read {}", display))?;
        let selected = extract_lines(&content, parsed.line_start, parsed.line_end);
        for line in &selected {
            println!("{}", line);
        }
    }
    Ok(())
}

// ── CLI ────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "kr", about = "Knowledge registry CLI — retrieve targeted knowledge from curated sources.\n\nURIs are stored relative to the .kr folder parent, displayed as ~/... or /... paths.")] 
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Comma-delimited list of kr folders to use (overrides mode)
    #[arg(long, value_delimiter = ',')]
    folder: Option<Vec<String>>,
}

#[derive(Subcommand)]
enum Commands {
    /// Registry operations
    Registry {
        #[command(subcommand)]
        cmd: RegistryCmd,
    },
    /// Source operations
    Source {
        #[command(subcommand)]
        cmd: SourceCmd,
    },
    /// Search across all sources in a registry
    Search {
        /// Registry name
        registry: String,
        /// Query string for rg
        query: String,
        /// Number of context lines (default: 2)
        #[arg(short, long, default_value_t = 2)]
        context: usize,
        /// Comma-separated tags to filter sources
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },
    /// Dump all content from a registry to stdout
    Dump {
        /// Registry name
        registry: String,
        /// Comma-separated tags to filter sources
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },
    /// Export registry to JSON file or stdout
    Export {
        /// Registry name
        registry: String,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import registry from JSON file or stdin
    Import {
        /// Input file (defaults to stdin)
        #[arg(short, long)]
        input: Option<String>,
    },
    /// Discover kr folders — show what kr will search across
    Discover,
}

#[derive(Subcommand)]
enum RegistryCmd {
    /// Create a new registry
    Create {
        /// Registry name
        name: String,
        /// Short description of the registry purpose (helps the model recall why this KR exists)
        #[arg(short, long)]
        description: Option<String>,
    },
    /// List all registries
    List,
    /// Show registry details
    Show {
        /// Registry name
        name: String,
    },
    /// Update registry metadata (description)
    Update {
        /// Registry name
        name: String,
        /// New description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a registry
    Delete {
        /// Registry name
        name: String,
    },
}

#[derive(Subcommand)]
enum SourceCmd {
    /// Add a source to a registry (supports glob patterns)
    Add {
        /// Registry name
        registry: String,
        /// URI or glob pattern (e.g. src/main.rs#L10-L42, ../shared/lib.rs, or src/models/*.rs).\nURIs stored relative to .kr parent, displayed as ~/... or /...
        uri: String,
        /// Human-readable label
        #[arg(short, long)]
        label: Option<String>,
        /// Comma-separated tags
        #[arg(short, long, value_delimiter = ',')]
        tags: Vec<String>,
    },
    /// List sources in a registry
    List {
        /// Registry name
        registry: String,
    },
    /// Remove a source by index (0-based)
    Remove {
        /// Registry name
        registry: String,
        /// Source index
        index: usize,
    },
    /// Update source metadata
    Update {
        /// Registry name
        registry: String,
        /// Source index
        index: usize,
        /// New label
        #[arg(short, long)]
        label: Option<String>,
        /// Comma-separated tags (replaces existing)
        #[arg(short, long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let folder_override = cli.folder.as_ref().map(|v| v.as_slice());

    match cli.command {
        Commands::Registry { cmd } => handle_registry(cmd)?,
        Commands::Source { cmd } => handle_source(cmd)?,
        Commands::Search { registry, query, context, tags } => {
            let reg = load_registry(&registry)?;
            search_registry(&reg, &query, context, &tags)?;
        }
        Commands::Dump { registry, tags } => {
            let reg = load_registry(&registry)?;
            dump_registry(&reg, &tags)?;
        }
        Commands::Export { registry, output } => {
            let reg = load_registry(&registry)?;
            let data = serde_json::to_string_pretty(&reg).context("serialize registry")?;
            if let Some(path) = output {
                fs::write(&path, &data).with_context(|| format!("write to {}", path))?;
                println!("✓ Exported to {}", path);
            } else {
                print!("{}", data);
            }
        }
        Commands::Import { input } => {
            let data = if let Some(path) = input {
                fs::read_to_string(&path).with_context(|| format!("read {}", path))?
            } else {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            };
            let reg: Registry = serde_json::from_str(&data).context("parse registry JSON")?;
            if registry_path(&reg.name).is_some() && registry_path(&reg.name).unwrap().exists() {
                anyhow::bail!("Registry '{}' already exists", reg.name);
            }
            save_registry(&reg)?;
            println!("✓ Imported registry '{}' with {} sources", reg.name, reg.sources.len());
        }
        Commands::Discover => {
            let discovery = discover_kr_folders(folder_override);
            println!("Mode: {}", discovery.mode);
            println!("Current folder: {}", display_path(&discovery.current_folder));
            println!();
            println!("Active kr folders:");
            for (i, f) in discovery.all_folders.iter().enumerate() {
                let marker = if i == 0 { "●" } else { "○" };
                println!("  {} {}", marker, display_path(f));
            }
            if !discovery.explicit_folders.is_empty() {
                println!();
                println!("Explicit folders (from .krrc):");
                for f in &discovery.explicit_folders {
                    println!("  • {}", display_path(f));
                }
            }
        }
    }

    Ok(())
}

fn handle_registry(cmd: RegistryCmd) -> Result<()> {
    match cmd {
        RegistryCmd::Create { name, description } => {
            if let Some(p) = registry_path(&name) {
                if p.exists() {
                    anyhow::bail!("Registry '{}' already exists", name);
                }
            }
            let reg = Registry {
                name,
                created: Utc::now().to_rfc3339(),
                description,
                sources: Vec::new(),
                kr_folder: registry_dir(),
            };
            save_registry(&reg)?;
            println!("✓ Created registry '{}' in {}", reg.name, display_path(&registry_dir()));
        }
        RegistryCmd::List => {
            let regs = list_registries()?;
            if regs.is_empty() {
                println!("No registries found.");
            } else {
                println!("{:<20} {:<10} {:<12} {}", "Name", "Sources", "Created", "Description");
                println!("{}", "-".repeat(80));
                for r in regs {
                    let created = &r.created[..10];
                    let desc = r.description.as_deref().unwrap_or("-");
                    println!("{:<20} {:<10} {:<12} {}", r.name, r.sources.len(), created, desc);
                }
            }
        }
        RegistryCmd::Show { name } => {
            let reg = load_registry(&name)?;
            println!("Registry:  {}", reg.name);
            println!("Created:   {}", reg.created);
            if let Some(ref desc) = reg.description {
                println!("Description: {}", desc);
            }
            println!("Sources:   {}", reg.sources.len());
            if !reg.sources.is_empty() {
                println!("\n{:<4} {:<12} {:<60}", "Idx", "Label", "Path");
                println!("{}", "-".repeat(80));
                for (i, s) in reg.sources.iter().enumerate() {
                    let label = s.label.as_deref().unwrap_or("-");
                    let resolved = resolve_uri(&reg.kr_folder, &s.uri);
                    println!("{:<4} {:<12} {}", i, label, display_path(&resolved));
                }
            }
        }
        RegistryCmd::Update { name, description } => {
            let mut reg = load_registry(&name)?;
            if let Some(desc) = description {
                reg.description = Some(desc);
            }
            save_registry(&reg)?;
            println!("✓ Updated registry '{}'", name);
        }
        RegistryCmd::Delete { name } => {
            if let Some(path) = registry_path(&name) {
                fs::remove_file(&path).with_context(|| format!("delete {}", path.display()))?;
                println!("✓ Deleted registry '{}'", name);
            } else {
                anyhow::bail!("Registry '{}' not found", name);
            }
        }
    }
    Ok(())
}

fn handle_source(cmd: SourceCmd) -> Result<()> {
    match cmd {
        SourceCmd::Add { registry, uri, label, tags } => {
            let mut reg = load_registry(&registry)?;
            let base_idx = reg.sources.len();

            if uri.contains('*') || uri.contains('?') {
                for entry in glob(&uri).context("glob pattern")? {
                    match entry {
                        Ok(path) => {
                            // Convert absolute path to relative URI for storage
                            let stored_uri = to_stored_uri(&reg.kr_folder, &path);
                            let source = Source {
                                uri: stored_uri,
                                label: label.clone(),
                                tags: tags.clone(),
                                added: Utc::now().to_rfc3339(),
                            };
                            reg.sources.push(source);
                        }
                        Err(e) => eprintln!("⚠  Glob error: {}", e),
                    }
                }
                save_registry(&reg)?;
                println!("✓ Added {} sources from glob", reg.sources.len() - base_idx);
            } else {
                // Validate the URI is parseable (catches bad line ranges, etc.)
                parse_uri(&uri).context("invalid URI format")?;

                // If user supplies file:///absolute, convert to relative
                let stored_uri = if let Some(rest) = uri.strip_prefix("file://") {
                    if rest.starts_with('/') {
                        // Absolute path — convert to relative
                        to_stored_uri(&reg.kr_folder, &PathBuf::from(rest))
                    } else {
                        uri.clone()
                    }
                } else {
                    uri.clone()
                };
                let source = Source {
                    uri: stored_uri,
                    label,
                    tags,
                    added: Utc::now().to_rfc3339(),
                };
                reg.sources.push(source);
                save_registry(&reg)?;
                println!("✓ Added source [{}]", reg.sources.len() - 1);
            }
        }
        SourceCmd::List { registry } => {
            let reg = load_registry(&registry)?;
            if reg.sources.is_empty() {
                println!("No sources in registry '{}'.", registry);
            } else {
                println!("{:<4} {:<15} {:<10} {}", "Idx", "Label", "Tags", "Path");
                println!("{}", "-".repeat(90));
                for (i, s) in reg.sources.iter().enumerate() {
                    let label = s.label.as_deref().unwrap_or("-");
                    let tags = s.tags.join(",");
                    let resolved = resolve_uri(&reg.kr_folder, &s.uri);
                    let path_display = display_path(&resolved);
                    println!("{:<4} {:<15} {:<10} {}", i, label, tags, path_display);
                }
            }
        }
        SourceCmd::Remove { registry, index } => {
            let mut reg = load_registry(&registry)?;
            if index >= reg.sources.len() {
                anyhow::bail!("Index {} out of range ({} sources)", index, reg.sources.len());
            }
            let removed = reg.sources.remove(index);
            let resolved = resolve_uri(&reg.kr_folder, &removed.uri);
            save_registry(&reg)?;
            println!("✓ Removed source: {}", display_path(&resolved));
        }
        SourceCmd::Update { registry, index, label, tags } => {
            let mut reg = load_registry(&registry)?;
            if index >= reg.sources.len() {
                anyhow::bail!("Index {} out of range ({} sources)", index, reg.sources.len());
            }
            if let Some(l) = label {
                reg.sources[index].label = Some(l);
            }
            if let Some(t) = tags {
                reg.sources[index].tags = t;
            }
            save_registry(&reg)?;
            println!("✓ Updated source [{}]", index);
        }
    }
    Ok(())
}