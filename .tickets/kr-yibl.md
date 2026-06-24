---
id: kr-yibl
status: open
deps: []
links: []
created: 2026-06-24T06:47:34Z
type: feature
priority: 2
assignee: memgrafter
tags: [config, discovery]
---
# Auto-discover .krrc from pwd up to home

## Notes

**2026-06-24T06:47:43Z**

.krrc is YAML with explicit kr folder paths. Walk pwd → home finding .krrc files, merge their folders. Lets projects have local kr storage alongside global ~/.kr/
