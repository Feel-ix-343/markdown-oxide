use std::sync::Arc;

use tracing::{info, instrument};

use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{CreateEmbeddingRequestArgs, EmbeddingInput},
    Client,
};

pub struct Embedder {
    client: Arc<Client<OpenAIConfig>>,
}

impl Embedder {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Client::new()),
        }
    }

    #[instrument(skip(self, text))]
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        info!("Generating embedding for text of length: {}", text.len());
        let request = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-3-large")
            .input(EmbeddingInput::String(text.to_string()))
            .build()?;

        let response = self.client.embeddings().create(request).await?;
        info!(
            "Received embedding response with {}",
            response.data[0].embedding.len()
        );

        Ok(response.data[0].embedding.clone())
    }

    #[instrument(skip(self, texts))]
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        info!("Generating embeddings for {} texts", texts.len());
        let batch_size = 2048;
        let mut all_embeddings = Vec::new();

        for chunk in texts.chunks(batch_size) {
            let request = CreateEmbeddingRequestArgs::default()
                .model("text-embedding-3-large")
                .input(EmbeddingInput::StringArray(chunk.to_vec()))
                .build()?;

            let response = self.client.embeddings().create(request).await?;
            info!(
                "Received embedding response with {} embeddings",
                response.data.len()
            );

            all_embeddings.extend(response.data.into_iter().map(|e| e.embedding));
        }

        Ok(all_embeddings)
    }
}
