import type { SiteDiagnosticReport } from './api';

export type MonitoringResult = { checkedAt: number; status: 'ok' | 'warning' | 'error' | 'unavailable'; summary: string; latencyMs: number | null };
export type MonitoringSite = { host: string; results: MonitoringResult[] };
export type MonitoringState = { version: 1; sites: MonitoringSite[] };
export type MonitoringCheck = (host: string) => Promise<SiteDiagnosticReport>;
export const MONITORING_STORAGE_KEY = 'nfqws2-ui:monitoring:v1';
export const MAX_MONITORING_SITES = 20;
export const MAX_MONITORING_RESULTS = 10;

export async function checkMonitoringHostsSequentially(hosts: string[], check: MonitoringCheck, onSuccess: (host: string, report: SiteDiagnosticReport) => void, onFailure: (host: string, error: unknown) => void): Promise<void> {
  for (const host of hosts) {
    try { onSuccess(host, await check(host)); }
    catch (error) { onFailure(host, error); }
  }
}

export function normalizeMonitoringHost(input: string): string | null {
  const host = input.trim().toLowerCase().replace(/\.$/, '');
  if (!host || host.includes('://') || /[\s/@?#:]/.test(host) || /^\d+(?:\.\d+){3}$/.test(host)) return null;
  if (host.length > 253 || !host.includes('.')) return null;
  const labels = host.split('.');
  if (labels.some((label) => !label || label.length > 63 || !/^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/.test(label))) return null;
  return host;
}

export function loadMonitoringState(storage: Storage | null): MonitoringState {
  const empty: MonitoringState = { version: 1, sites: [] };
  try {
    const raw = storage?.getItem(MONITORING_STORAGE_KEY);
    if (!raw) return empty;
    const value = JSON.parse(raw) as Partial<MonitoringState>;
    if (value.version !== 1 || !Array.isArray(value.sites)) return empty;
    const sites = value.sites.flatMap((site): MonitoringSite[] => {
      if (!site || typeof site.host !== 'string' || !Array.isArray(site.results)) return [];
      const host = normalizeMonitoringHost(site.host);
      if (!host) return [];
      const results = site.results.filter((r): r is MonitoringResult => Boolean(r && typeof r === 'object' && typeof r.checkedAt === 'number' && ['ok', 'warning', 'error', 'unavailable'].includes(r.status) && typeof r.summary === 'string' && (typeof r.latencyMs === 'number' || r.latencyMs === null))).slice(0, MAX_MONITORING_RESULTS);
      return [{ host, results }];
    }).slice(0, MAX_MONITORING_SITES);
    return { version: 1, sites };
  } catch { return empty; }
}

export function saveMonitoringState(storage: Storage | null, state: MonitoringState): void {
  try { storage?.setItem(MONITORING_STORAGE_KEY, JSON.stringify({ version: 1, sites: state.sites.slice(0, MAX_MONITORING_SITES).map((site) => ({ ...site, results: site.results.slice(0, MAX_MONITORING_RESULTS) })) })); } catch { /* storage is optional */ }
}

export function appendMonitoringResult(state: MonitoringState, host: string, report: SiteDiagnosticReport | null, checkedAt: number, failureMessage = 'Проверка не выполнена'): MonitoringState {
  const normalized = normalizeMonitoringHost(host);
  if (!normalized) return state;
  const status = report?.current.status ?? 'unavailable';
  const result: MonitoringResult = { checkedAt, status, summary: report?.summary ?? failureMessage, latencyMs: report?.current.durationMs ?? null };
  const sites = [...state.sites];
  const index = sites.findIndex((site) => site.host === normalized);
  const previous = index >= 0 ? sites.splice(index, 1)[0] : { host: normalized, results: [] };
  sites.unshift({ host: normalized, results: [result, ...previous.results].slice(0, MAX_MONITORING_RESULTS) });
  return { version: 1, sites: sites.slice(0, MAX_MONITORING_SITES) };
}
