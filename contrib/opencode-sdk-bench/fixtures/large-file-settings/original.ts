export interface AppSetting {
  key: string;
  value: string;
}

export const SETTINGS: AppSetting[] = [
  { key: "feature.alpha", value: "on" },
  { key: "feature.beta", value: "off" },
  { key: "feature.gamma", value: "on" },
  { key: "feature.delta", value: "off" },
  { key: "feature.epsilon", value: "on" },
  { key: "feature.zeta", value: "off" },
  { key: "feature.eta", value: "on" },
  { key: "feature.theta", value: "off" },
  { key: "feature.iota", value: "on" },
  { key: "feature.kappa", value: "off" },
  { key: "feature.lambda", value: "on" },
  { key: "feature.mu", value: "off" },
  { key: "feature.nu", value: "on" },
  { key: "feature.xi", value: "off" },
  { key: "feature.omicron", value: "on" },
  { key: "feature.pi", value: "off" },
  { key: "feature.rho", value: "on" },
  { key: "feature.sigma", value: "off" },
  { key: "feature.tau", value: "on" },
  { key: "feature.upsilon", value: "off" },
  { key: "feature.phi", value: "on" },
  { key: "feature.chi", value: "off" },
  { key: "feature.psi", value: "on" },
  { key: "feature.omega", value: "off" },
  { key: "service.timeoutMs", value: "12000" },
  { key: "service.maxInflight", value: "64" },
  { key: "service.retries", value: "5" },
  { key: "service.backoffMs", value: "250" },
  { key: "service.bulk.enabled", value: "true" },
  { key: "service.bulk.batch", value: "200" },
  { key: "ui.theme", value: "light" },
  { key: "ui.locale", value: "en-US" },
  { key: "ui.dateFormat", value: "iso" },
  { key: "cache.enabled", value: "true" },
  { key: "cache.ttl", value: "300" },
  { key: "cache.maxEntries", value: "5000" },
];

export function getSetting(key: string): string | undefined {
  return SETTINGS.find((s) => s.key === key)?.value;
}
