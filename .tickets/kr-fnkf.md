---
id: kr-fnkf
status: open
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
