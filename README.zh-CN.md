# Token Proxy

[English](README.md) | 中文

中转 ai api 的工具，比如转发 openai api 格式，gemini ai api 格式，claude ai api 格式，在本地运行，用于统计总 token 用量的、也可以负载均衡、优先级之类的

## 配置说明

- 配置文件：`config.jsonc`（支持注释与尾随逗号）。
- 位置：Tauri 配置目录。
- 保存会重写文件；仅保留文件头部注释块。

示例：

```jsonc
{
  "host": "127.0.0.1",
  "port": 9208,
  "local_api_key": null,
  "log_path": "proxy.log",
  "enable_api_format_conversion": false,
  "upstream_strategy": "priority_round_robin",
  "upstreams": [
    {
      "id": "openai-default",
      "provider": "openai",
      "base_url": "https://api.openai.com",
      "api_key": null,
      "priority": 0,
      "index": 0,
      "enabled": true
    },
    {
      "id": "openai-responses",
      "provider": "openai-response",
      "base_url": "https://api.openai.com",
      "api_key": null,
      "priority": 0,
      "index": 1,
      "enabled": true
    },
    {
      "id": "claude-default",
      "provider": "claude",
      "base_url": "https://api.anthropic.com",
      "api_key": null,
      "priority": 0,
      "index": 2,
      "enabled": true
    }
  ]
}
```

说明：
- 路由规则内置：`/v1/chat/completions` → `openai`，`/v1/responses` → `openai-response`，`/v1/messages`（及子路径）/`/v1/complete` → `claude`；OpenAI Chat/Responses 互转由 `enable_api_format_conversion` 控制（默认：`false`）。Claude 不做格式转换。
- Claude 鉴权使用 `x-api-key`；当请求未携带 `anthropic-version` 时，代理默认补 `2023-06-01`（可被请求头覆盖）。
- `priority` 越大优先级越高；同优先级内按 `index` 升序。
- `index` 缺失时，保存配置会在当前最大 `index` 之后按顺序全局自动补齐。
- `enabled` 用于禁用某个 upstream 而不删除；禁用的 upstream 不参与负载均衡。
