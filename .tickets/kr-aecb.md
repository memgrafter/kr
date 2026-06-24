---
id: kr-aecb
status: open
deps: [kr-yibl]
links: []
created: 2026-06-24T06:47:34Z
type: feature
priority: 2
assignee: memgrafter
tags: [cli, folder]
---
# CLI flag to specify kr folder(s) explicitly

## Notes

**2026-06-24T06:47:43Z**

Example: kr --folder /path/to/kr search X query or kr --folder ~/.kr,/path/to/project/.kr list. Comma-delimited list of folders to search.

**2026-06-24T06:50:18Z**

Comma-delimited list of specific kr folders: kr --folder ~/.kr,/path/to/project/.kr search X query. Overrides mode when specified.
