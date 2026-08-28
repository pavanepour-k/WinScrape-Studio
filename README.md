# WinScrape Studio

Natural-language-driven web scraping automation for Windows.

Describe what you want to scrape in plain English, review the generated
plan, and export the results to CSV, JSON, XLSX, or Parquet.

## Building

```
cargo build --release
```

Default build includes the GUI (`ui` feature) and HTTP-only scraping
(`http-only` feature - no headless browser).

### Optional features

| Feature      | Adds                                                              | Notes |
|--------------|--------------------------------------------------------------------|-------|
| `ui`         | The desktop GUI (`winscrape-studio` binary)                        | On by default |
| `browser`    | Headless-browser scraping fallback via Playwright                  | Requires the Playwright Node.js driver to be installed separately |
| `api`        | A local REST API server (see `src/api.rs`)                         | Off by default |
| `local-llm`  | In-process natural-language processing via a local GGUF model      | Off by default - see below |
| `full`       | `ui` + `browser` + `api`                                           | |

`wss-cli`, the command-line binary, is always built alongside the GUI.

## Natural language processing

Describing what to scrape works in one of three ways, tried in this
order, each falling back to the next automatically:

1. **Local GGUF model (`local-llm` feature)** - fully offline, no
   network call, no external process. Build with:
   ```
   cargo build --release --features local-llm
   ```
   Then download a compatible GGUF model yourself (this app does not
   download models) and point Settings → Natural Language Processing →
   "Model file" at it, and enable "Enable local GGUF inference". The
   model needs a llama.cpp-style BPE ("gpt2") tokenizer embedded in the
   GGUF file - most current Llama 3.x / Qwen2.x / Mistral GGUF
   conversions qualify; older SentencePiece-tokenizer GGUFs (e.g. the
   original Llama 2 conversions) do not.

   This is the least-tested part of the app - it was written against
   the actual `candle-core`/`candle-transformers` 0.11.0 source, but
   has not been run end-to-end (no GPU and no way to download a
   multi-gigabyte model in the environment it was built in). If you hit
   build or runtime errors with this feature, they're expected to be
   narrow (a signature mismatch in `src/llm/candle_backend.rs`), not
   structural - please report them.

2. **Local Ollama server** - install [Ollama](https://ollama.com), pull
   a model (`ollama pull llama3.2`), and it's used automatically
   whenever it's reachable at `http://localhost:11434` (configurable in
   Settings). Also free, but runs as a separate background process
   rather than in-process.

3. **Rule-based fallback** - a built-in keyword/pattern matcher that
   needs no model, no download, and no setup at all. This is what runs
   if neither of the above is available, so **the app is fully usable
   with zero AI/LLM setup** - natural-language descriptions will just be
   interpreted more literally/simply than with a real model.

No cloud API keys or per-request costs are involved in any of the three
paths above.

## Data & config locations

Configuration, the local SQLite database, and (if used) local GGUF
models live under your OS's standard app-data directory for
`com.winscrape.studio` (see `directories::ProjectDirs` in
`src/config/mod.rs`).
