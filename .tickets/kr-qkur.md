---
id: kr-qkur
status: closed
deps: [kr-64zs]
links: []
created: 2026-06-24T17:47:53Z
type: task
priority: 3
assignee: memgrafter
tags: [test]
---
# Test suite: resolution, display, backward compat

## Notes

**2026-06-24T17:48:19Z**

Add tests/features.rs or new test file:

- resolve_uri with ~/.kr parent → resolves ~/code/project/src/main.rs correctly
- resolve_uri with /project/.kr parent → resolves /project/src/main.rs correctly  
- resolve_uri with file:///absolute backward compat → returns absolute as-is
- display_path under home → shows ~/...
- display_path outside home → shows /...
- dump output uses ~/ in headers (integration test)
- search output uses ~/ in headers (integration test)
- glob registration stores relative URIs (check JSON content)

**2026-06-24T18:04:24Z**

9 tests in tests/resolution.rs: resolve relative URIs, display ~/ format, backward compat, storage format verification, full integration flow.
