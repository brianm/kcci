# Keith's Card Catalog Index

## .plan

1. parse inputs in common formats to build candidate list
2. use openlibrary to get information about candidates
3. create embedding for each candidate
4. store embedding raw data with the candidate in sqlite or something (so we can rebuild index without recaclulating embeddings)
5. Build an index (annoy? faiss?) of the embeddings

## References

* [Library of Congress API](https://loc.gov/apis)
* [OpenLibrary](https://openlibrary.org) [API](https://openlibrary.org/developers/api)
* [WorldCat](https://www.oclc.org/developer/api/oclc-apis/worldcat-search-api.en.html0)
* [Google Books API](https://developers.google.com/books/)
* [SBERT Semantic Search](https://www.sbert.net/examples/applications/semantic-search/README.html)
* [Simon Willison's Explorations](https://til.simonwillison.net/python/gtr-t5-large)
* [Semantic Search in Rust](https://sachaarbonel.medium.com/how-to-build-a-semantic-search-engine-in-rust-e96e6378cfd9)
* [Annoy](https://github.com/spotify/annoy)
* [FAISS](https://github.com/facebookresearch/faiss)
  * To save the index: `faiss.write_index(index, filename)`
  * To load the index: `index = faiss.read_index(filename)`
* [Textual](https://textual.textualize.io) terminal ui library
* [FastEmbed-rs](https://github.com/anush008/fastembed-rs) way to generate embeddings in rust
* [sqlite-vec](https://github.com/asg017/sqlite-vec) vector search in sqlite
* [lancedb](https://lancedb.github.io/lancedb/) embedded search-oriented database
  * Note need to [statically link lzma-sys](https://github.com/lancedb/lancedb/tree/main/rust/lancedb)


