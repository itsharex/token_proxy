# Token Proxy

English | [中文](README.zh-CN.md)

A tool for proxying AI APIs, such as forwarding OpenAI API format, Gemini AI API format, Claude AI API format, running locally, used for counting total token usage, and also for load balancing, priority management, and similar functions.

## Configuration

- Config file: `config.jsonc` (JSONC with comments and trailing commas).
- Location: Tauri config directory.
- Saving rewrites the file; the leading comment block is preserved.

Example:

```jsonc
{
  "host": "127.0.0.1",
  "port": 9208,
  "local_api_key": null,
  "log_path": "proxy.log",
  "upstream_strategy": "priority_round_robin",
  "upstreams": [
    {
      "id": "openai-default",
      "provider": "openai",
      "base_url": "https://api.openai.com",
      "api_key": null,
      "priority": 0,
      "index": 0
    },
    {
      "id": "openai-responses",
      "provider": "openai-response",
      "base_url": "https://api.openai.com",
      "api_key": null,
      "priority": 0,
      "index": 1
    }
  ]
}
```

Notes:
- Request format routing is built in: `/v1/chat/completions` prefers provider `openai`, `/v1/responses` prefers provider `openai-response`. When the preferred provider is missing, the proxy will translate between Chat Completions and Responses formats automatically.
- `priority` sorts descending; `index` sorts ascending inside the same priority group.
- Missing `index` values are auto-assigned globally after the current max index when saving.
