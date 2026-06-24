---
name: knowledge-registry
description: Retrieve targeted knowledge from curated sources — register specific files or line ranges as knowledge artifacts, then search or dump them to build focused agent context. KR and KRs refer to knowledge registries.
---

Register specific files or line ranges as curated knowledge — not the whole codebase, that's what git is for. Then retrieve from them — search with rg or dump content — to build focused agent context.

## Why use it

- Retrieve from multiple files/ranges in one command instead of repeating paths
- Search across registered sources with `kr search <reg> "query"`
- Dump all content to stdout for piping into agent prompts
- Line-range support — register only the relevant sections of large files

## When to use

- You need to retrieve information from a curated set of files — search or dump as a unit
- Building focused context for a coding task by retrieving across multiple registered sources
- Searching within a subset of a codebase instead of the whole repo

## When NOT to use

- You need the whole codebase — that's what git is for; register only curated knowledge, not every file
- One-off single-file searches — just use `rg` directly
- You need remote/URL sources — only local filesystem supported now

## Usage

```bash
kr registry create <name> -d "Purpose of this KR"   # always add a description
kr source add <reg> file:///path/to/file.rs#L1-L50 --label "main" --tags "core"
kr source list <reg>
kr search <reg> "query" -c 2          # rg with context lines
kr dump <reg>                         # all content to stdout
kr registry update <name> -d "Updated purpose"  # update description
kr source update <reg> 0 --label "new" --tags "updated"
kr source remove <reg> 0
kr registry delete <name>
```

URI formats: `file:///path.rs` (full file), `file:///path.rs#L10-L42` (range), `file:///path.rs#L10` (from line 10 to end).

**Path resolution**: URIs are stored relative to the `.kr` folder parent. At display time, paths resolve to `~/...` if under home, `/...` otherwise — no brittle absolute paths in the registry file.

**Cross-registration**: A source can belong to multiple KRs if it's relevant to both domains. Overlap is intentional cross-referencing, not duplication.

## Examples

```bash
# Register curated knowledge for an auth task — specific sections, not the whole repo
kr registry create auth-knowledge -d "Auth module: handler code + design notes"
kr source add auth-knowledge file:///path/to/app/src/auth.rs#L1-L80 --label "auth module"
kr source add auth-knowledge file:///path/to/app/DESIGN.md#L15-L30 --label "auth design notes"
kr search auth-knowledge "fn authenticate"
kr dump auth-knowledge > context.md
```

## Output

- `search`: rg results grouped by source with headers
- `dump`: line-numbered content per source with URI/label headers

**Description**: Every KR should have a description (set with `-d` on create or update). This helps the model recall the purpose of a KR when it returns to it in a later session.

Cost: local only, zero network. Benefit: clean context management across multi-file tasks.
