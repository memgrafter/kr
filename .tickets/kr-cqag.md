---
id: kr-cqag
status: closed
deps: []
links: []
created: 2026-06-24T21:04:03Z
type: bug
priority: 2
assignee: memgrafter
tags: [safety, testing]
---
# Test isolation: use dedicated temp .kr folder

Tests write directly to real ~/.kr/ with no cleanup on panic. Need global setup/teardown or cwd override so test registries never pollute real data.
