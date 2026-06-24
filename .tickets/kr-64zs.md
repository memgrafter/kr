---
id: kr-64zs
status: closed
deps: []
links: []
created: 2026-06-24T17:47:03Z
type: feature
priority: 1
assignee: memgrafter
tags: [storage, portability, internal]
---
# Store URIs relative to .kr parent, resolve at read time for display

## Notes

**2026-06-24T17:47:14Z**

Storage: URIs in JSON always relative to parent of .kr folder. E.g. registry at /project/.kr/my.json stores 'src/main.rs#L1-L30'. Registry at ~/.kr/my.json stores 'code/project/src/main.rs'.

Display: resolve relative URI against .kr parent → absolute path → format as ~/... if under home, /... otherwise. No root field in JSON, inferred from file location.

Key changes:
1. Registry struct gains kr_folder: PathBuf (computed from file path at load time, not stored)
2. resolve_uri(kr_folder, uri) → absolute PathBuf
3. display_path(absolute) → ~/... or /... string
4. search/dump/source list use resolved paths for file ops, display paths for output
5. Glob registration: expand glob in CWD → absolute → convert to relative to .kr parent → store
6. Backward compat: detect file:/// + absolute path at load time, use as-is (lazy)

**2026-06-24T17:47:48Z**

## Acceptance Criteria

### Storage
- [ ] Source URIs stored as relative paths in JSON (no file:// prefix, no absolute paths)
  - Registry at /project/.kr/my.json → stores 'src/main.rs#L1-L30'
  - Registry at ~/.kr/my.json → stores 'code/project/src/main.rs'
- [ ] No root field in JSON — kr_folder inferred from registry file location
- [ ] Line-range fragments (#L1-L30) preserved as-is after the relative path

### Resolution (read time)
- [ ] resolve_uri(kr_folder, uri) joins .kr parent with relative URI → absolute PathBuf
- [ ] display_path(absolute) formats as ~/... if under home, /... otherwise
- [ ] Backward compat: existing file:///absolute URIs detected at load time, used as-is without rewrite

### Display
- [ ] dump headers show display_path (~/ or / format), never raw absolute with PII
- [ ] search output shows display_path in source headers
- [ ] source list shows resolved paths for readability
- [ ] kr discover shows folders using ~/ format where applicable

### Registration
- [ ] Normal add: user supplies relative path like 'src/main.rs' — stored as-is
  - If user supplies file:///absolute, convert to relative at store time
- [ ] Glob add: expand in CWD → absolute → convert each to relative to .kr parent → store
- [ ] ../ and ~/ in URIs handled correctly (../ joins normally, ~ expanded at load time before join)

### Remote URIs
- [ ] https://, git+ssh:// pass through unchanged — not resolved, not displayed as file paths

### Tests
- [ ] Test resolve_uri with home-relative kr_folder (~/.kr)
- [ ] Test resolve_uri with project kr_folder (/project/.kr)
- [ ] Test display_path strips home prefix → ~/ format
- [ ] Test display_path for non-home path → / format
- [ ] Test dump output uses display_path in headers
- [ ] Test search output uses display_path in headers
- [ ] Test glob registration stores relative URIs
- [ ] Test backward compat: file:///absolute URI used as-is without error

**2026-06-24T18:05:31Z**

All 5 subtasks closed. 93/93 tests pass stable.
