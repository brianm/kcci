pub mod ingest;

#[cfg(test)]
mod tests {
    use rust_bert::pipelines::sentence_embeddings::{
        SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
    };

    #[test]
    fn berty() {
        let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL12V2)
            .create_model()
            .unwrap();

        let sentences = ["this is an example sentence", "each sentence is converted"];

        let _output = model.encode(&sentences);
    }
}
