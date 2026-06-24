---
id: kr-t39m
status: closed
deps: []
links: []
created: 2026-06-24T21:04:03Z
type: bug
priority: 1
assignee: memgrafter
tags: [safety]
---
# Guard registry delete: name validation

Reject registry names containing / \ .. or control characters to prevent path traversal attacks. Currently kr registry delete '../etc/passwd' could resolve outside the .kr folder.
