---
yatl_version: 1
title: Add database schema migrations
id: m5m78nhv
created: 2025-12-31T04:29:50.885280Z
updated: 2025-12-31T04:29:59.380759Z
author: Brian McCallister
priority: high
tags:
- infrastructure
---

Need a migration system to handle schema changes without requiring full re-sync.

Current pain points:
- Added subjects column to FTS - required manual ALTER TABLE + FTS rebuild
- Any future schema changes will have similar issues

Options to consider:
- Simple version number in DB + migration functions in Rust
- refinery crate (embedded migrations)
- sqlx migrations
- Custom approach with schema version table

Key requirements:
- Run automatically on app startup
- Handle FTS virtual table recreation (can't ALTER)
- Idempotent migrations

---
# Log: 2025-12-31T04:29:50Z Brian McCallister

Created task.
