---
id: kr-wqdq
status: closed
deps: []
links: []
created: 2026-06-24T21:04:03Z
type: bug
priority: 1
assignee: memgrafter
tags: [safety]
---
# Guard registry delete: content verification

Read the JSON file and verify the name field matches before deleting. Prevents deleting a maliciously placed .json file that happens to share a name with a registry.
