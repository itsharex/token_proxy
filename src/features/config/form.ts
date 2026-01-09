import {
  type ConfigForm,
  type ModelMappingForm,
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
    modelMappings: [],
  },
  {
    id: "openai-responses",
    provider: "openai-response",
    baseUrl: "https://api.openai.com",
    apiKey: "",
    priority: "0",
    index: "1",
    enabled: true,
    modelMappings: [],
  },
  {
    id: "anthropic-default",
    provider: "anthropic",
    baseUrl: "https://api.anthropic.com",
    apiKey: "",
    priority: "0",
    index: "2",
    enabled: true,
    modelMappings: [],
  },
  {
    id: "gemini-default",
    provider: "gemini",
    baseUrl: "https://generativelanguage.googleapis.com",
    apiKey: "",
    priority: "0",
    index: "3",
    enabled: true,
    modelMappings: [],
  },
];

const INTEGER_PATTERN = /^-?\d+$/;
let modelMappingCounter = 0;

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
    modelMappings: [],
  };
}

export function createModelMapping(pattern = "", target = "") {
  // 稳定 id 用于列表渲染，避免输入时因 key 变化导致焦点丢失
  modelMappingCounter += 1;
  return {
    id: `model-mapping-${Date.now()}-${modelMappingCounter}`,
    pattern,
    target,
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
      modelMappings: toModelMappingForm(upstream.model_mappings),
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
      model_mappings: toModelMappingPayload(upstream.modelMappings),
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
    const mappingError = validateModelMappings(upstream.modelMappings, id);
    if (mappingError) {
      return { valid: false, message: mappingError };
    }
  }

  return { valid: true, message: "" };
}

function toModelMappingForm(mappings: Record<string, string>): ModelMappingForm[] {
  return Object.entries(mappings).map(([pattern, target]) =>
    createModelMapping(pattern, target),
  );
}

function toModelMappingPayload(mappings: ModelMappingForm[]) {
  const entries = mappings.map(
    (mapping): [string, string] => [mapping.pattern.trim(), mapping.target.trim()],
  );
  return Object.fromEntries(entries);
}

function validateModelMappings(mappings: ModelMappingForm[], upstreamId: string) {
  const seen = new Set<string>();
  let wildcardSeen = false;
  for (let index = 0; index < mappings.length; index += 1) {
    const row = String(index + 1);
    const pattern = mappings[index]?.pattern.trim() ?? "";
    const target = mappings[index]?.target.trim() ?? "";
    if (!pattern) {
      return m.error_model_mapping_pattern_required({ id: upstreamId, row });
    }
    if (!target) {
      return m.error_model_mapping_target_required({ id: upstreamId, row });
    }
    if (seen.has(pattern)) {
      return m.error_model_mapping_pattern_duplicate({ id: upstreamId, pattern });
    }
    seen.add(pattern);

    if (pattern === "*") {
      if (wildcardSeen) {
        return m.error_model_mapping_wildcard_multiple({ id: upstreamId });
      }
      wildcardSeen = true;
      continue;
    }

    if (pattern.includes("*") && !pattern.endsWith("*")) {
      return m.error_model_mapping_pattern_invalid({ id: upstreamId, pattern });
    }

    if (pattern.endsWith("*")) {
      const prefix = pattern.slice(0, -1).trim();
      if (!prefix) {
        return m.error_model_mapping_prefix_required({ id: upstreamId, row });
      }
      if (prefix.includes("*")) {
        return m.error_model_mapping_pattern_invalid({ id: upstreamId, pattern });
      }
    }
  }
  return "";
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
