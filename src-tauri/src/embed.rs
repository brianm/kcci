use ndarray::Array2;
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use std::path::Path;
use std::sync::Mutex;
use tokenizers::Tokenizer;

use crate::error::{KcciError, Result};

/// Embedding dimension for multi-qa-mpnet-base-cos-v1
pub const EMBEDDING_DIM: usize = 768;

/// Cached embedder for reuse across calls
static EMBEDDER: Mutex<Option<EmbedderInner>> = Mutex::new(None);

/// Inner embedder that holds the session
struct EmbedderInner {
    tokenizer: Tokenizer,
    session: Session,
}

impl EmbedderInner {
    fn load(model_dir: &Path) -> Result<Self> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let model_path = model_dir.join("model.onnx");

        if !tokenizer_path.exists() || !model_path.exists() {
            return Err(KcciError::Onnx(format!(
                "Model not found at {:?}. Expected tokenizer.json and model.onnx",
                model_dir
            )));
        }

        let tokenizer = Tokenizer::from_file(&tokenizer_path)?;

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .commit_from_file(&model_path)?;

        Ok(Self { tokenizer, session })
    }

    fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        // Tokenize
        let encoding = self.tokenizer.encode(text, true)?;

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();
        let seq_len = input_ids.len();

        // Create input tensors
        let input_ids_array = Array2::from_shape_vec((1, seq_len), input_ids)
            .map_err(|e| KcciError::Onnx(format!("Failed to create input_ids array: {}", e)))?;
        let attention_array = Array2::from_shape_vec((1, seq_len), attention_mask.clone())
            .map_err(|e| {
                KcciError::Onnx(format!("Failed to create attention_mask array: {}", e))
            })?;

        // Convert to ort Values
        let input_ids_value = Value::from_array(input_ids_array)?;
        let attention_value = Value::from_array(attention_array)?;

        // Run inference with named inputs
        let outputs = self.session.run(ort::inputs![
            "input_ids" => input_ids_value,
            "attention_mask" => attention_value
        ])?;

        // Get token embeddings: shape [1, seq_len, 768]
        let token_embs = outputs[0]
            .try_extract_array::<f32>()
            .map_err(|e| KcciError::Onnx(format!("Failed to extract tensor: {}", e)))?;

        // Mean pooling with attention mask
        let mut sum = vec![0.0f32; EMBEDDING_DIM];
        let mut mask_sum = 0.0f32;

        for i in 0..seq_len {
            let mask = attention_mask[i] as f32;
            mask_sum += mask;
            for j in 0..EMBEDDING_DIM {
                sum[j] += token_embs[[0, i, j]] * mask;
            }
        }

        // Avoid division by zero
        let mask_sum = mask_sum.max(1e-9);
        for v in &mut sum {
            *v /= mask_sum;
        }

        // L2 normalize
        let norm: f32 = sum.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut sum {
                *v /= norm;
            }
        }

        Ok(sum)
    }
}

/// Initialize the embedder if not already initialized
pub fn init_embedder(model_dir: &Path) -> Result<()> {
    let mut guard = EMBEDDER.lock().unwrap();
    if guard.is_none() {
        *guard = Some(EmbedderInner::load(model_dir)?);
    }
    Ok(())
}

/// Generate embedding for text using the global embedder
pub fn embed_text(text: &str) -> Result<Vec<f32>> {
    let mut guard = EMBEDDER.lock().unwrap();
    let embedder = guard
        .as_mut()
        .ok_or_else(|| KcciError::Onnx("Embedder not initialized".to_string()))?;
    embedder.embed(text)
}

/// Combine book fields into text for embedding
pub fn get_embedding_text(title: &str, authors: &[String], description: &str) -> String {
    let mut parts = vec![title.to_string()];
    if !authors.is_empty() {
        parts.push(format!("by {}", authors.join(", ")));
    }
    if !description.is_empty() {
        parts.push(description.to_string());
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_embedding_text() {
        let text = get_embedding_text(
            "The Great Book",
            &["John Doe".to_string()],
            "A wonderful story",
        );
        assert_eq!(text, "The Great Book by John Doe A wonderful story");

        let text_no_author = get_embedding_text("Title Only", &[], "Description here");
        assert_eq!(text_no_author, "Title Only Description here");

        let text_no_desc = get_embedding_text("Book", &["Author".to_string()], "");
        assert_eq!(text_no_desc, "Book by Author");
    }
}
