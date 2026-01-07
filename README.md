# Token Proxy

English | [中文](README.zh-CN.md)

A tool for proxying AI APIs, such as forwarding OpenAI API format, Gemini AI API format, Claude AI API format, running locally, used for counting total token usage, and also for load balancing, priority management, and similar functions.

## Configuration

- Config file: `config.jsonc` (JSONC with comments and trailing commas).
- Location: Tauri config directory.
- Saving rewrites the file; the leading comment block is preserved.
