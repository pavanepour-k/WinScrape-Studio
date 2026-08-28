//! In-process local LLM inference via `candle`, loading a quantized GGUF
//! model file directly - no external server, no API key, no network
//! call, and no per-request cost. Only compiled in with
//! `--features local-llm`.
//!
//! This is the least-verified code in the project: it was written by
//! downloading and reading the actual `candle-core` 0.11.0 /
//! `candle-transformers` 0.11.0 source from static.crates.io to confirm
//! function signatures (`ModelWeights::from_gguf`, `LogitsProcessor`,
//! `TokenizerFromGguf`, GGUF metadata access), since guessing at a
//! version-sensitive inference API without checking would be too risky.
//! That said, it has not been compiled or run - there was no way to do a
//! real end-to-end test in the sandbox this was built in (no GPU, and
//! downloading a multi-gigabyte GGUF model plus building candle from
//! scratch wasn't practical there). Build this feature and try it before
//! relying on it; if `cargo build --features local-llm` reports errors
//! here, they should be narrow (a signature drift in one of the calls
//! below), not structural.
//!
//! What this needs from the user: a GGUF model file downloaded
//! separately (this app does not fetch models) with a llama.cpp-style
//! BPE/"gpt2" tokenizer embedded (most current Llama 3.x/Qwen2.x/Mistral
//! GGUF conversions qualify; older SentencePiece-tokenizer GGUFs like the
//! original Llama 2 conversions do not - `Tokenizer::from_gguf` will
//! return an error for those, which this module treats as "unavailable"
//! and falls back from, same as any other failure here).

use anyhow::{Context, Result};
use candle_core::quantized::gguf_file;
use candle_core::quantized::tokenizer::TokenizerFromGguf;
use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::quantized_llama::ModelWeights;
use tokenizers::Tokenizer;
use tracing::{debug, info, warn};

use crate::config::LLMConfig;

/// Generate text from a prompt using a local GGUF model. Returns the
/// generated continuation (not including the prompt). Any failure along
/// the way (missing file, unsupported tokenizer, generation error) is
/// returned as `Err` - the caller (`LLMProcessor`) treats that as "local
/// inference unavailable right now" and falls back to Ollama or the
/// rule-based generator, so this never has to be perfect to be useful.
pub fn generate(config: &LLMConfig, prompt: &str) -> Result<String> {
    if !config.model_path.exists() {
        anyhow::bail!(
            "GGUF model file not found at {}",
            config.model_path.display()
        );
    }

    info!("Loading GGUF model from {}", config.model_path.display());
    let device = Device::Cpu;

    let mut file = std::fs::File::open(&config.model_path)
        .with_context(|| format!("opening {}", config.model_path.display()))?;
    let content = gguf_file::Content::read(&mut file)
        .with_context(|| format!("reading GGUF metadata from {}", config.model_path.display()))?;

    // Build the tokenizer from the GGUF file's own embedded vocabulary
    // *before* handing `content` (by value) to `ModelWeights::from_gguf`
    // below, and pull the EOS token id out of the same metadata map
    // while we still have it.
    let tokenizer = Tokenizer::from_gguf(&content)
        .context("building tokenizer from GGUF metadata (unsupported/missing tokenizer info)")?;
    let eos_token_id = content
        .metadata
        .get("tokenizer.ggml.eos_token_id")
        .and_then(|v| v.to_u32().ok());

    let mut model = ModelWeights::from_gguf(content, &mut file, &device)
        .context("building model weights from GGUF content")?;
    info!("GGUF model loaded");

    let encoding = tokenizer
        .encode(prompt, true)
        .map_err(|e| anyhow::anyhow!("tokenizer encode failed: {e}"))?;
    let prompt_tokens = encoding.get_ids().to_vec();
    if prompt_tokens.is_empty() {
        anyhow::bail!("prompt tokenized to zero tokens");
    }

    let mut logits_processor = LogitsProcessor::new(
        config.candle_seed,
        Some(config.temperature.max(0.0) as f64),
        Some(0.9),
    );

    let mut all_tokens = prompt_tokens.clone();
    let mut generated_tokens: Vec<u32> = Vec::new();

    // Feed the whole prompt through first (index_pos 0, seq_len =
    // prompt length) to prime the KV cache, then generate one token at a
    // time (index_pos advances by 1 each step since each subsequent
    // input is a single new token).
    let prompt_tensor = Tensor::new(prompt_tokens.as_slice(), &device)
        .context("building prompt tensor")?
        .unsqueeze(0)
        .context("unsqueezing prompt tensor")?;
    let mut logits = model
        .forward(&prompt_tensor, 0)
        .context("forward pass over prompt")?;
    let mut index_pos = prompt_tokens.len();

    const REPEAT_PENALTY: f32 = 1.1;
    const REPEAT_LAST_N: usize = 64;

    for _ in 0..config.max_tokens {
        let last_logits = logits
            .squeeze(0)
            .context("squeezing logits")?
            .squeeze(0)
            .context("squeezing logits")?;

        let start_at = all_tokens.len().saturating_sub(REPEAT_LAST_N);
        let penalized = candle_transformers::utils::apply_repeat_penalty(
            &last_logits,
            REPEAT_PENALTY,
            &all_tokens[start_at..],
        )
        .context("applying repeat penalty")?;

        let next_token = logits_processor
            .sample(&penalized)
            .context("sampling next token")?;

        if Some(next_token) == eos_token_id {
            debug!("Hit EOS token after {} generated tokens", generated_tokens.len());
            break;
        }

        all_tokens.push(next_token);
        generated_tokens.push(next_token);

        let next_input = Tensor::new(&[next_token], &device)
            .context("building next-token tensor")?
            .unsqueeze(0)
            .context("unsqueezing next-token tensor")?;
        logits = model
            .forward(&next_input, index_pos)
            .context("forward pass over next token")?;
        index_pos += 1;
    }

    if generated_tokens.is_empty() {
        warn!("Model generated no tokens before stopping");
    }

    let text = tokenizer
        .decode(&generated_tokens, true)
        .map_err(|e| anyhow::anyhow!("tokenizer decode failed: {e}"))?;

    Ok(text)
}
