---
id: kr-9yom
status: closed
deps: [kr-64zs]
links: []
created: 2026-06-24T17:47:53Z
type: task
priority: 3
assignee: memgrafter
tags: [display, output]
---
# Update search/dump/source list to use resolved + display paths

## Notes

**2026-06-24T17:48:14Z**

Every place that reads a source URI and uses it for file I/O or display must change:

search_registry: resolve_uri → absolute path for rg. display_path in header output (── ~/src/main.rs ──)
dump_registry: resolve_uri → absolute path for file read. display_path in header (// ~/src/main.rs [L1-L30] — label)
source list: show resolved display_path instead of raw URI
kr discover: format folder paths with ~/ where applicable

**2026-06-24T17:57:23Z**

search/dump headers use display_path. source list/registry show use resolved paths. discover uses ~/ format. All display paths consistent.
