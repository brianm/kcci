---
yatl_version: 1
title: Design database schema migration strategy
id: zp2qhgae
created: 2025-12-31T04:15:49.057773Z
updated: 2025-12-31T04:16:00.148099Z
author: Brian McCallister
priority: high
tags:
- backend
- database
- architecture
---

Need a strategy for evolving the SQLite database schema as the application grows.

**Considerations:**
- Users will have existing databases with data they want to preserve
- Migrations need to run automatically on app startup
- Should handle version tracking (what schema version is this DB?)
- Need rollback strategy or at least backup-before-migrate

**Options to explore:**
1. **Embedded migrations** - SQL files bundled in the app, run sequentially by version number
2. **refinery/diesel migrations** - Rust crates for managing migrations
3. **sqlx migrations** - If switching to sqlx from rusqlite
4. **Custom versioned approach** - Simple user_version pragma + handwritten upgrade functions

**Current state:**
- Schema defined in src-tauri/src/db/schema.sql
- No migration tooling currently in place
- Need to understand current schema before designing migrations

---
# Log: 2025-12-31T04:15:49Z Brian McCallister

Created task.
