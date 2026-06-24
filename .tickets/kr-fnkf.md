---
id: kr-fnkf
status: closed
deps: [kr-64zs]
links: []
created: 2026-06-24T17:47:53Z
type: task
priority: 2
assignee: memgrafter
tags: [storage, registration]
---
# Store relative URIs in source add (normal + glob)

## Notes

**2026-06-24T17:48:10Z**

Normal add: if user types 'file:///absolute/path.rs', strip file:// and convert to relative to .kr parent. If user types 'src/main.rs', store as-is.

Glob add: currently stores 'file:///absolute/expanded/path'. Change to: expand glob → absolute path → make relative to kr_folder.parent() → store relative URI.

Edge case: if the resolved path is outside the .kr parent (e.g. ~/other/file.rs from a /project/.kr registry), use ~ expansion so stored URI is '~/other/file.rs'.

**2026-06-24T17:57:17Z**

source add converts file:///absolute → relative via to_stored_uri. Glob expands in CWD, stores relative URIs. parse_uri validation kept for bad line ranges.
