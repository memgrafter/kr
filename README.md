# Knowledge Registry CLI

A Rust CLI for registering, searching, and managing knowledge sources — files, file ranges, URIs — to build context windows for coding agent sessions.

## Problem

Coding agents need focused context. Scrolling through entire repos wastes tokens and attention. You want to say "here are the 5 files (or file+line ranges) relevant to this task" and have a clean way to search across them as a unit.

## Design

### Core Concepts

- **Registry** — a named collection of knowledge sources, stored as a JSON file on disk
- **Source** — a URI-addressable entity: `file:///path/to/file.rs`, `file:///path/to/file.rs#L10-L42`, or eventually `https://...`, `git+ssh://...`
- **Query** — search across all sources in a registry, delegating to tools like `rg` for local files

### Data Model

```jsonc
{
  "name": "my-registry",
  "created": "2025-06-23T...",
  "sources": [
    {
      "uri": "src/main.rs#L1-L80",       // stored relative to .kr parent
      "label": "main entry point",
      "tags": ["entrypoint", "app"],
      "added": "2025-06-23T..."
    }
  ]
}
```

### URI Scheme

URIs are stored **relative to the `.kr` folder parent** — no brittle absolute paths. At display time, paths resolve to `~/...` if under home, `/...` otherwise.

```
src/main.rs                            # entire file (relative)
src/main.rs#L10-L42                     # line range
src/main.rs#L10                         # single line
../shared/lib.rs                        # cross-project reference
~/other-project/file.rs                 # home-relative reference
file:///absolute/path.rs                # accepted for backward compat, stored as relative
```

### CLI Commands

```
kr registry create <name>          # create a new registry
kr registry list                   # list all registries
kr registry delete <name>          # remove a registry
kr registry show <name>            # display registry contents

kr source add <registry> <uri> [--label L] [--tags t1,t2]   # add source
kr source list <registry>                                         # list sources
kr source remove <registry> <index>                               # remove source
kr source update <registry> <index> [--label L] [--tags t1,t2]   # update metadata

kr search <registry> <query> [--context N]    # rg across all sources
kr dump <registry>                            # dump all content to stdout (for piping)
```

### Architecture

```
┌─────────────┐     ┌──────────────┐     ┌──────────────┐
│  CLI (clap) │────▶│  Domain      │────▶│  Storage     │
│             │◀────│  (models+    │◀────│  (JSON files) │
└─────────────┘     │   commands)  │     └──────────────┘
                    └──────────────┘
                          │
                    ┌─────▼─────┐
                    │  Search   │
                    │  (rg, etc)│
                    └───────────┘
```

### Extension Points

1. **Remote sources** — `https://`, `git+ssh://` URIs fetched on demand
2. **Caching** — downloaded content cached with TTL
3. **Filters** — tag-based filtering before search/dump
4. **Export formats** — Markdown, JSON, agent-prompt format

## Getting Started

```bash
cargo run -- registry create my-app
cargo run -- source add my-app file:///path/to/my-app/src/main.rs#L1-L50 --label "main" --tags "entrypoint"
cargo run -- search my-app "fn main"
cargo run -- dump my-app
```
