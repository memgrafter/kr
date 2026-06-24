use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ── Domain Models ──────────────────────────────────────────────

/// A URI-addressable knowledge source with optional line range and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Source {
    uri: String,
    label: Option<String>,
    tags: Vec<String>,
    added: String,
}

/// A named collection of knowledge sources.
#[derive(Debug, Serialize, Deserialize)]
struct Registry {
    name: String,
    created: String,
    sources: Vec<Source>,
}

// ── Storage ────────────────────────────────────────────────────

fn registry_dir() -> PathBuf {
    let dir = dirs::home_dir()
        .expect("cannot find home directory")
        .join(".kr");
    fs::create_dir_all(&dir).expect("cannot create .kr directory");
    dir
}

fn registry_path(name: &str) -> PathBuf {
    registry_dir().join(format!("{name}.json"))
}

fn save_registry(registry: &Registry) -> Result<()> {
    let path = registry_path(&registry.name);
    let data = serde_json::to_string_pretty(registry).context("serialize registry")?;
    fs::write(&path, data).with_context(|| format!("write to {}", path.display()))
}

fn load_registry(name: &str) -> Result<Registry> {
    let path = registry_path(name);
    let data = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&data).context("parse registry JSON")
}

fn list_registries() -> Result<Vec<Registry>> {
    let dir = registry_dir();
    let mut registries = Vec::new();
    for entry in fs::read_dir(&dir).context("read registry directory")? {
        let entry = entry.context("read directory entry")?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            if let Ok(reg) = load_registry(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
            ) {
                registries.push(reg);
            }
        }
    }
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
                // L1-L30 or L1-L L30
                line_start = Some(s.parse().context("parse start line")?);
                line_end = Some(rest.trim_start_matches('L').parse().context("parse end line")?);
            } else {
                // Single line: L42
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

fn uri_to_file_path(parsed: &ParsedUri) -> Option<PathBuf> {
    if parsed.scheme == "file" {
        parsed.path.as_ref().map(PathBuf::from)
    } else {
        None
    }
}

// ── Search ─────────────────────────────────────────────────────

fn search_registry(registry: &Registry, query: &str, context: usize) -> Result<()> {
    let mut file_targets: Vec<(PathBuf, Option<usize>, Option<usize>)> = Vec::new();

    for source in &registry.sources {
        let parsed = parse_uri(&source.uri).context(format!("parse URI {}", source.uri))?;
        if let Some(path) = uri_to_file_path(&parsed) {
            if path.exists() {
                file_targets.push((path, parsed.line_start, parsed.line_end));
            } else {
                eprintln!("⚠  File not found: {}", path.display());
            }
        }
    }

    if file_targets.is_empty() {
        println!("No local file sources to search.");
        return Ok(());
    }

    // Build rg command for each file
    for (path, start, end) in &file_targets {
        let path_str = path.to_string_lossy().to_string();

        // If there's a line range, pipe through sed first to limit search scope
        let (input_cmd, input_arg) = match (start, end) {
            (Some(s), Some(e)) => {
                let mut cmd = std::process::Command::new("sed");
                cmd.arg("-n").arg(format!("{},{}p", s, e)).arg(path_str.clone());
                let sed_out = cmd.output().context("run sed")?;
                let lines = String::from_utf8_lossy(&sed_out.stdout);
                // Search within extracted lines via rg reading from stdin
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
                    println!("\n── {} (L{}-L{}) ──", path.display(), s, e);
                    print!("{}", stdout);
                }
                continue;
            }
            (Some(s), None) => {
                let mut cmd = std::process::Command::new("sed");
                cmd.arg("-n");
                cmd.arg(format!("{},$p", s)); // from line s to end
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
                    println!("\n── {} (L{}+) ──", path.display(), s);
                    print!("{}", stdout);
                }
                continue;
            }
            _ => ("rg".to_string(), path_str.clone()),
        };

        let mut cmd = std::process::Command::new(&input_cmd);
        cmd.arg("--context")
            .arg(context.to_string())
            .arg(query)
            .arg(&input_arg);

        let output = cmd.output().context("run rg")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            println!("\n── {} ──", path.display());
            print!("{}", stdout);
        }
    }

    Ok(())
}

// ── Dump ───────────────────────────────────────────────────────

fn dump_registry(registry: &Registry) -> Result<()> {
    for source in &registry.sources {
        let parsed = parse_uri(&source.uri).context(format!("parse URI {}", source.uri))?;
        if let Some(path) = uri_to_file_path(&parsed) {
            if !path.exists() {
                eprintln!("⚠  File not found: {}", path.display());
                continue;
            }

            println!("\n// ── {} ──", source.uri);
            if let Some(ref label) = source.label {
                println!("//   {}", label);
            }

            let content = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
            let lines: Vec<&str> = content.lines().collect();

            if let (Some(start), Some(end)) = (parsed.line_start, parsed.line_end) {
                // Clamp to valid range
                let start_idx = (start - 1).min(lines.len());
                let end_idx = end.min(lines.len());
                for i in start_idx..end_idx {
                    println!("L{:>4} | {}", i + 1, lines[i]);
                }
            } else if let Some(start) = parsed.line_start {
                let start_idx = (start - 1).min(lines.len());
                for i in start_idx..lines.len() {
                    println!("L{:>4} | {}", i + 1, lines[i]);
                }
            } else {
                for (i, line) in lines.iter().enumerate() {
                    println!("L{:>4} | {}", i + 1, line);
                }
            }
        }
    }
    Ok(())
}

// ── CLI ────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "kr", about = "Knowledge registry CLI for managing context sources")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    },
    /// Dump all content from a registry to stdout
    Dump {
        /// Registry name
        registry: String,
    },
}

#[derive(Subcommand)]
enum RegistryCmd {
    /// Create a new registry
    Create {
        /// Registry name
        name: String,
    },
    /// List all registries
    List,
    /// Show registry details
    Show {
        /// Registry name
        name: String,
    },
    /// Delete a registry
    Delete {
        /// Registry name
        name: String,
    },
}

#[derive(Subcommand)]
enum SourceCmd {
    /// Add a source to a registry
    Add {
        /// Registry name
        registry: String,
        /// URI (e.g. file:///path/to/file.rs#L10-L42)
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

    match cli.command {
        Commands::Registry { cmd } => handle_registry(cmd)?,
        Commands::Source { cmd } => handle_source(cmd)?,
        Commands::Search { registry, query, context } => {
            let reg = load_registry(&registry)?;
            search_registry(&reg, &query, context)?;
        }
        Commands::Dump { registry } => {
            let reg = load_registry(&registry)?;
            dump_registry(&reg)?;
        }
    }

    Ok(())
}

fn handle_registry(cmd: RegistryCmd) -> Result<()> {
    match cmd {
        RegistryCmd::Create { name } => {
            if registry_path(&name).exists() {
                anyhow::bail!("Registry '{}' already exists", name);
            }
            let reg = Registry {
                name,
                created: Utc::now().to_rfc3339(),
                sources: Vec::new(),
            };
            save_registry(&reg)?;
            println!("✓ Created registry '{}'", reg.name);
        }
        RegistryCmd::List => {
            let regs = list_registries()?;
            if regs.is_empty() {
                println!("No registries found.");
            } else {
                println!("{:<20} {:<10} {}", "Name", "Sources", "Created");
                println!("{}", "-".repeat(52));
                for r in regs {
                    let created = &r.created[..10]; // just the date
                    println!("{:<20} {:<10} {}", r.name, r.sources.len(), created);
                }
            }
        }
        RegistryCmd::Show { name } => {
            let reg = load_registry(&name)?;
            println!("Registry: {}", reg.name);
            println!("Created:  {}", reg.created);
            println!("Sources:  {}", reg.sources.len());
            if !reg.sources.is_empty() {
                println!("\n{:<4} {:<12} {:<60}", "Idx", "Label", "URI");
                println!("{}", "-".repeat(80));
                for (i, s) in reg.sources.iter().enumerate() {
                    let label = s.label.as_deref().unwrap_or("-");
                    println!("{:<4} {:<12} {}", i, label, s.uri);
                }
            }
        }
        RegistryCmd::Delete { name } => {
            let path = registry_path(&name);
            fs::remove_file(&path).with_context(|| format!("delete {}", path.display()))?;
            println!("✓ Deleted registry '{}'", name);
        }
    }
    Ok(())
}

fn handle_source(cmd: SourceCmd) -> Result<()> {
    match cmd {
        SourceCmd::Add { registry, uri, label, tags } => {
            let mut reg = load_registry(&registry)?;
            // Validate URI parses
            parse_uri(&uri).context("invalid URI format")?;
            let source = Source {
                uri,
                label,
                tags,
                added: Utc::now().to_rfc3339(),
            };
            reg.sources.push(source);
            save_registry(&reg)?;
            println!("✓ Added source [{}]", reg.sources.len() - 1);
        }
        SourceCmd::List { registry } => {
            let reg = load_registry(&registry)?;
            if reg.sources.is_empty() {
                println!("No sources in registry '{}'.", registry);
            } else {
                println!("{:<4} {:<15} {:<10} {}", "Idx", "Label", "Tags", "URI");
                println!("{}", "-".repeat(90));
                for (i, s) in reg.sources.iter().enumerate() {
                    let label = s.label.as_deref().unwrap_or("-");
                    let tags = s.tags.join(",");
                    println!("{:<4} {:<15} {:<10} {}", i, label, tags, s.uri);
                }
            }
        }
        SourceCmd::Remove { registry, index } => {
            let mut reg = load_registry(&registry)?;
            if index >= reg.sources.len() {
                anyhow::bail!("Index {} out of range ({} sources)", index, reg.sources.len());
            }
            let removed = reg.sources.remove(index);
            save_registry(&reg)?;
            println!("✓ Removed source: {}", removed.uri);
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
