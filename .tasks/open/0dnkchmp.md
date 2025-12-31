---
yatl_version: 1
title: Audit app state combinations and ensure graceful UX for all states
id: 0dnkchmp
created: 2025-12-31T04:20:21.986038Z
updated: 2025-12-31T04:20:34.199983Z
author: Brian McCallister
priority: high
tags:
- ux
- architecture
- design
---

Walk through all possible app states and their combinations to ensure users can always take useful actions and aren't blocked from progressing.

**Independent state dimensions:**
1. **Books**: none | some
2. **Enrichment**: none | partial | complete  
3. **Model**: not downloaded | downloaded
4. **Embeddings**: none | partial | complete

**Key principles:**
- User should always have a clear action available
- State transitions should not block each other
- Example: User skips embeddings initially, later downloads model → should auto-generate embeddings for existing books

**State combinations to analyze:**
| Books | Enriched | Model | Embeddings | What can user do? |
|-------|----------|-------|------------|-------------------|
| none  | -        | no    | -          | Import books      |
| some  | no       | no    | no         | Enrich, Browse    |
| some  | yes      | no    | no         | Browse, Download model |
| some  | yes      | yes   | no         | Generate embeddings, Browse |
| some  | yes      | yes   | yes        | Full functionality |
| ...etc for all combinations...

**Questions:**
- Should embedding generation be automatic when model becomes available?
- How to surface 'next step' guidance to user in each state?
- Should we show a setup wizard / progress indicator?

**Related task:** 392xdz2y (Default to Browse when no embeddings)

---
# Log: 2025-12-31T04:20:21Z Brian McCallister

Created task.
