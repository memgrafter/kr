---
id: kr-5cbg
status: closed
deps: []
links: []
created: 2026-06-24T06:41:51Z
type: feature
priority: 3
assignee: memgrafter
tags: [portability, sharing]
---
# Import/export registries between sessions and machines

## Notes

**2026-06-24T06:42:06Z**

Registries live in ~/.kr/ with no way to share or port. Need: kr export > file.json, kr import < file.json, or kr clone from remote registry.

**2026-06-24T06:58:42Z**

kr export X -o file.json, kr import -i file.json. Also supports stdin/stdout.
