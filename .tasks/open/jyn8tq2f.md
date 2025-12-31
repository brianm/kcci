---
yatl_version: 1
title: Add additional metadata sources beyond Open Library
id: jyn8tq2f
created: 2025-12-31T04:18:24.643353Z
updated: 2025-12-31T04:18:35.051594Z
author: Brian McCallister
priority: medium
tags:
- backend
- enrichment
- feature
---

Open Library has gaps in coverage - many books return no metadata. Add fallback sources to improve enrichment success rate.

**Potential sources:**
- **Library of Congress** - Authoritative, good for older/academic titles
- **Google Books API** - Large coverage, may have rate limits
- **Search engines** - Last resort, scrape structured data
- **ISBN databases** - isbndb.com, etc.
- **Amazon Product API** - If we have ASINs already, could query directly

**Implementation approach:**
- Current enrichment: src-tauri/src/enrich.rs (OpenLibrary client)
- Could implement as a chain of sources: try OL first, then fallbacks
- Need to normalize data from different sources into common format
- Consider caching/storing which source provided the data

**Considerations:**
- API rate limits and keys for some services
- Data quality/consistency across sources
- Prioritization order (which source is most reliable?)

---
# Log: 2025-12-31T04:18:24Z Brian McCallister

Created task.
