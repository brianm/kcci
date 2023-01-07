use tracing;
use tracing::instrument;
pub mod ingest;

#[instrument]
pub fn add(left: usize, right: usize) -> usize {
    tracing::debug!("adding {} and {}", left, right);
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_bert::pipelines::sentence_embeddings::{
        SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
    };

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn berty() {
        let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL12V2)
            .create_model().unwrap();

        let sentences = ["this is an example sentence", "each sentence is converted"];

        let output = model.predict(&sentences);
    }
}
