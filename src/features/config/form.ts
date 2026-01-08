import {
  type ConfigForm,
  type ProxyConfigFile,
  type UpstreamForm,
  UPSTREAM_STRATEGIES,
} from "@/features/config/types";
import { m } from "@/paraglide/messages.js";

const DEFAULT_UPSTREAMS: UpstreamForm[] = [
  {
    id: "openai-default",
    provider: "openai",
    baseUrl: "https://api.openai.com",
    apiKey: "",
    priority: "0",
    index: "0",
    enabled: true,
  },
  {
    id: "openai-responses",
    provider: "openai-response",
    baseUrl: "https://api.openai.com",
    apiKey: "",
    priority: "0",
    index: "1",
    enabled: true,
  },
  {
    id: "anthropic-default",
    provider: "anthropic",
    baseUrl: "https://api.anthropic.com",
    apiKey: "",
    priority: "0",
    index: "2",
    enabled: true,
  },
  {
    id: "gemini-default",
    provider: "gemini",
    baseUrl: "https://generativelanguage.googleapis.com",
    apiKey: "",
    priority: "0",
    index: "3",
    enabled: true,
  },
];

const INTEGER_PATTERN = /^-?\d+$/;

export const EMPTY_FORM: ConfigForm = {
  host: "127.0.0.1",
  port: "9208",
  localApiKey: "",
  logPath: "proxy.log",
  enableApiFormatConversion: false,
  upstreamStrategy: UPSTREAM_STRATEGIES[0].value,
  upstreams: DEFAULT_UPSTREAMS.map((upstream) => ({ ...upstream })),
};

export function createEmptyUpstream(): UpstreamForm {
  return {
    id: "",
    provider: "",
    baseUrl: "",
    apiKey: "",
    priority: "",
    index: "",
    enabled: true,
  };
}

export function toForm(config: ProxyConfigFile): ConfigForm {
  return {
    host: config.host,
    port: String(config.port),
    localApiKey: config.local_api_key ?? "",
    logPath: config.log_path,
    enableApiFormatConversion: config.enable_api_format_conversion,
    upstreamStrategy: config.upstream_strategy,
    upstreams: config.upstreams.map((upstream) => ({
      id: upstream.id,
      provider: upstream.provider,
      baseUrl: upstream.base_url,
      apiKey: upstream.api_key ?? "",
      priority: upstream.priority === null ? "" : String(upstream.priority),
      index: upstream.index === null ? "" : String(upstream.index),
      enabled: upstream.enabled,
    })),
  };
}

export function toPayload(form: ConfigForm): ProxyConfigFile {
  const port = Number.parseInt(form.port, 10);
  return {
    host: form.host.trim(),
    port,
    local_api_key: form.localApiKey.trim() ? form.localApiKey.trim() : null,
    log_path: form.logPath.trim(),
    enable_api_format_conversion: form.enableApiFormatConversion,
    upstream_strategy: form.upstreamStrategy,
    upstreams: form.upstreams.map((upstream) => ({
      id: upstream.id.trim(),
      provider: upstream.provider.trim(),
      base_url: upstream.baseUrl.trim(),
      api_key: upstream.apiKey.trim() ? upstream.apiKey.trim() : null,
      priority: parseOptionalInt(upstream.priority),
      index: parseOptionalInt(upstream.index),
      enabled: upstream.enabled,
    })),
  };
}

export function validate(form: ConfigForm) {
  if (!form.host.trim()) {
    return { valid: false, message: m.error_host_required() };
  }
  const port = Number.parseInt(form.port, 10);
  if (!Number.isFinite(port) || port < 1 || port > 65535) {
    return { valid: false, message: m.error_port_range() };
  }
  if (!form.logPath.trim()) {
    return { valid: false, message: m.error_log_path_required() };
  }
  if (!form.upstreams.length) {
    return { valid: false, message: m.error_upstream_at_least_one() };
  }
  const hasEnabledUpstream = form.upstreams.some((upstream) => upstream.enabled);
  if (!hasEnabledUpstream) {
    return { valid: false, message: m.error_upstream_at_least_one_enabled() };
  }

  const ids = new Set<string>();
  for (const upstream of form.upstreams) {
    const id = upstream.id.trim();
    if (!id) {
      return { valid: false, message: m.error_upstream_id_required() };
    }
    if (ids.has(id)) {
      return { valid: false, message: m.error_upstream_id_unique({ id }) };
    }
    ids.add(id);
    const provider = upstream.provider.trim();
    if (!provider) {
      return { valid: false, message: m.error_upstream_provider_required({ id }) };
    }
    if (!upstream.baseUrl.trim()) {
      return { valid: false, message: m.error_upstream_base_url_required({ id }) };
    }
    if (!isValidOptionalInt(upstream.priority)) {
      return { valid: false, message: m.error_upstream_priority_integer({ id }) };
    }
    if (!isValidOptionalInt(upstream.index)) {
      return { valid: false, message: m.error_upstream_index_integer({ id }) };
    }
  }

  return { valid: true, message: "" };
}

function isValidOptionalInt(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return true;
  }
  return INTEGER_PATTERN.test(trimmed);
}

function parseOptionalInt(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  if (!INTEGER_PATTERN.test(trimmed)) {
    return null;
  }
  const number = Number.parseInt(trimmed, 10);
  return Number.isFinite(number) ? number : null;
}
