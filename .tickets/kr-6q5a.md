---
id: kr-6q5a
status: closed
deps: []
links: []
created: 2026-06-24T21:04:03Z
type: bug
priority: 1
assignee: memgrafter
tags: [safety]
---
# Guard registry delete: path confinement

Verify the resolved path starts with one of the known .kr directories before deleting. Defense-in-depth against path traversal even if name validation is bypassed.
