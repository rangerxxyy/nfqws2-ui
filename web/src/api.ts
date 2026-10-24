export type Locale = 'ru' | 'en' | 'zh' | 'tr';
export type StatusResponse = {
  service: { state: string; detail: string; checkedAt: number | null; version: string | null };
  system: { memoryTotalBytes: number | null; memoryUsedBytes: number | null; cpuPercent: number | null; source?: 'keenetic-rci' | 'proc' };
  panel: { version: string; port: number };
};
export type ConfigResponse = { file: string; content: string; revision: string };
export type ConfigValidationDiagnostic = { severity: 'error' | 'warning'; code: string; message: string; line: number | null };
export type ConfigValidationResponse = { valid: boolean; diagnostics: ConfigValidationDiagnostic[]; checks: { parser: 'passed' | 'failed'; installedBinary: 'not_run' } };
export type SiteDiagnosticStep = { id: string; status: 'ok' | 'warning' | 'error' | 'unavailable'; detail: string; durationMs: number | null; httpCode: number | null };
export type SiteDiagnosticReport = { host: string; addresses: string[]; steps: SiteDiagnosticStep[]; summary: string; limitations: string[]; direct: SiteDiagnosticStep; current: SiteDiagnosticStep; quic: null | { direct: SiteDiagnosticStep; current: SiteDiagnosticStep; method: 'quic_initial' | 'http3'; http3Advertised: boolean | null } };
export type SiteTracePhase = { state: 'complete' | 'error' | 'unavailable'; ok: boolean | null; code: number | null; durationMs: number | null; diagnosticClassification?: string | null; reason: string };
export type SiteTraceReport = { host: string; address: string | null; withoutNfqws2: SiteTracePhase; withNfqws2: SiteTracePhase; traceLines: string[]; traceTruncated: boolean; traceHostname: string | null; traceProfile: string | null; traceActions: string[]; traceWarnings: string[]; summary: string };
export type SiteTraceOperation = { id: string; state: 'queued' | 'running' | 'stopping' | 'complete' | 'cancelled' | 'failed'; stage: string; message: string; report: SiteTraceReport | null; updatedAt: number };
export type StrategyPickResult = { candidate: { name: string; source: string; steps: string[] }; attempts: number; successes: number; confirmed: boolean; elapsedMs: number; detail: string };
export type StrategyPickOperation = { id: string; host: string; protocol: string; state: 'queued' | 'running' | 'stopping' | 'complete' | 'partial' | 'no_match' | 'site_unblocked' | 'cancelled' | 'failed'; stage: string; current: string | null; done: number; total: number; results: StrategyPickResult[]; message: string; updatedAt: number };
export type StrategyPickPreview = { operationId: string; candidateIndex: number; host: string; protocol: 'http' | 'https'; candidate: StrategyPickResult['candidate']; originalRevision: string; originalContent: string; proposedContent: string; summary: string };
export type FreezeStrategyMatch = { sni: string; step: string; attempts: number; successes: number; averageMs: number | null; reusedFrom: string | null };
export type FreezeCandidateAttempt = { sni: string; step: string; attempts: number; successes: number; averageMs: number | null; confirmed: boolean; reason: string | null };
export type FreezeNetworkResult = { label: string; host: string; path: string; targetKind: 'host_reference' | 'pinned_address' | 'user_host'; address: string | null; asnHint: number | null; asn: { state: 'verified'; asn: number } | { state: 'changed'; expected: number; actual: number } | { state: 'unavailable' }; direct: { classification: string; ok: boolean; bytesReceived: number | null; replies: number | null; elapsedMs: number; reason: string }; current: { classification: string; ok: boolean; bytesReceived: number | null; replies: number | null; elapsedMs: number; reason: string }; freezeDetected: boolean; triedCandidates: number; candidateTotal: number; foundStrategy: FreezeStrategyMatch | null; candidateAttempts: FreezeCandidateAttempt[] };
export type FreezeScanOperation = { id: string; state: 'queued' | 'running' | 'stopping' | 'complete' | 'partial' | 'cancelled' | 'failed'; stage: string; current: string | null; done: number; total: number; results: FreezeNetworkResult[]; message: string; updatedAt: number };
export type NfqwsRuntimeProfile = { index: number; name: string | null; filters: string[]; strategies: string[]; payloads: string[] };
export type NfqwsRuntime = { running: boolean; pid: number | null; source: 'proc-cmdline'; profiles: NfqwsRuntimeProfile[]; limitations: string[] };
export type NfqwsListId = 'user.list' | 'auto.list' | 'exclude.list' | 'ipset.list' | 'ipset_exclude.list';
export type NfqwsListSummary = { id: NfqwsListId; path: string; readable: boolean; writable: boolean };
export type NfqwsListResponse = { id: NfqwsListId; path: string; content: string; revision: string };
export type InstallationPath = {
  path: string;
  exists: boolean;
  readable: boolean;
  writable: boolean;
  usedByActiveConfig: boolean;
};
export type NfqwsModuleReference = {
  kind: 'lua' | 'blob';
  argument: string;
  path: string | null;
  resolvedPath: string | null;
  exists: boolean;
  readable: boolean;
  resolution: 'absolute' | 'dynamic' | 'relative_unknown';
};
export type NfqwsInstallation = {
  platform: 'entware' | 'openwrt' | 'unknown';
  installationRoot?: string | null;
  detectedBinary?: InstallationPath | null;
  networkInterfaces: string[];
  config: InstallationPath | null;
  configCandidates: InstallationPath[];
  initScript: InstallationPath | null;
  initScriptCandidates: InstallationPath[];
  binaryCandidates: InstallationPath[];
  lists: InstallationPath[];
  moduleReferences?: NfqwsModuleReference[];
};
export type BackupItem = { id: string; createdAt: number; file: string };
export type PanelRelease = { tag: string; publishedAt: string | null; notes: string };
export type UiUpdateStatus = { currentVersion: string; latestVersion: string | null; updateAvailable: boolean; releaseUrl: string | null; checkedAt: number | null };
export type NfqwsCoreReleases = { releases: PanelRelease[]; installedTag: string | null };
export type UpdateOperation = {
  id: string;
  state: 'downloading' | 'verifying' | 'installing' | 'health_check' | 'success' | 'rolled_back' | 'failed';
  message?: string;
};

export class ApiError extends Error {
  constructor(readonly status: number, message: string) { super(message); this.name = 'ApiError'; }
}

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.body && !headers.has('Content-Type')) headers.set('Content-Type', 'application/json');
  const response = await fetch(path, { ...init, headers, credentials: 'same-origin' });
  const payload = await response.json().catch(() => ({})) as { error?: string };
  if (!response.ok) throw new ApiError(response.status, payload.error || `HTTP ${response.status}`);
  return payload as T;
}

export const nfqwsLists = {
  index: () => api<{ files: NfqwsListSummary[] }>('/api/nfqws2/lists'),
  get: (id: NfqwsListId) => api<NfqwsListResponse>(`/api/nfqws2/lists/${encodeURIComponent(id)}`),
  save: (id: NfqwsListId, content: string, revision: string) => api<NfqwsListResponse>(
    `/api/nfqws2/lists/${encodeURIComponent(id)}`,
    { method: 'PUT', body: JSON.stringify({ content, revision }) },
  ),
};

export const nfqwsInstallation = {
  get: () => api<NfqwsInstallation>('/api/nfqws2/installation'),
};

export const nfqwsRuntime = {
  get: () => api<NfqwsRuntime>('/api/nfqws2/runtime'),
};

export const nfqwsBackups = {
  list: () => api<{ backups: BackupItem[] }>('/api/backups'),
  create: () => api<BackupItem>('/api/backups', { method: 'POST' }),
  restore: (id: string) => api<{ message: string }>(`/api/backups/${encodeURIComponent(id)}/restore`, { method: 'POST' }),
};

export const nfqwsApply = {
  apply: (content: string, revision: string) => api<{ config: ConfigResponse; message: string }>(
    '/api/nfqws2/apply',
    { method: 'POST', body: JSON.stringify({ content, revision }) },
  ),
};

export const nfqwsConfig = {
  validate: (content: string) => api<ConfigValidationResponse>('/api/nfqws2/config/validate', {
    method: 'POST', body: JSON.stringify({ content }),
  }),
  preview: (content: string, key: string, value: string) => api<{ content: string; changedVariable: string }>(
    '/api/nfqws2/config/preview',
    { method: 'POST', body: JSON.stringify({ content, key, value }) },
  ),
};

export const nfqwsDiagnostics = {
  site: (host: string) => api<SiteDiagnosticReport>('/api/nfqws2/diagnostics/site', {
    method: 'POST', body: JSON.stringify({ host }),
  }),
  traceStart: (host: string, proto: 'http' | 'https') => api<SiteTraceOperation>('/api/nfqws2/diagnostics/trace', {
    method: 'POST', body: JSON.stringify({ host, proto }),
  }),
  traceStatus: (id: string) => api<SiteTraceOperation>(`/api/nfqws2/diagnostics/trace/operations/${encodeURIComponent(id)}`),
  traceStop: (id: string) => api<SiteTraceOperation>(`/api/nfqws2/diagnostics/trace/operations/${encodeURIComponent(id)}/stop`, { method: 'POST' }),
  freezeScanStart: (extraHosts: string[]) => api<FreezeScanOperation>('/api/nfqws2/diagnostics/freeze-scan', { method: 'POST', body: JSON.stringify({ extraHosts }) }),
  freezeScanStatus: (id: string) => api<FreezeScanOperation>(`/api/nfqws2/diagnostics/freeze-scan/operations/${encodeURIComponent(id)}`),
  freezeScanCancel: (id: string) => api<FreezeScanOperation>(`/api/nfqws2/diagnostics/freeze-scan/operations/${encodeURIComponent(id)}/cancel`, { method: 'POST' }),
  strategyStart: (host: string, protocol: 'http' | 'https', mode: 'quick' | 'extended' | 'full' | 'custom', repeats = 3, customSteps?: string[], stopAfterFirstConfirmed = false, includeCommunity = false, includeOtherHistory = false, refineLighter = false) => api<StrategyPickOperation>('/api/nfqws2/strategies/pick', { method: 'POST', body: JSON.stringify({ host, protocol, mode, repeats, stopAfterFirstConfirmed, includeCommunity, includeOtherHistory, refineLighter, ...(customSteps ? { customSteps } : {}) }) }),
  strategyStatus: (id: string) => api<StrategyPickOperation>(`/api/nfqws2/strategies/pick/operations/${encodeURIComponent(id)}`),
  strategyCancel: (id: string) => api<StrategyPickOperation>(`/api/nfqws2/strategies/pick/operations/${encodeURIComponent(id)}/cancel`, { method: 'POST' }),
  strategyHistory: () => api<StrategyPickOperation[]>('/api/nfqws2/strategies/pick/history'),
  strategyPreview: (id: string, candidateIndex: number) => api<StrategyPickPreview>(`/api/nfqws2/strategies/pick/operations/${encodeURIComponent(id)}/preview`, { method: 'POST', body: JSON.stringify({ candidateIndex }) }),
  freezeStrategyPreview: (id: string, resultIndex: number) => api<StrategyPickPreview>('/api/nfqws2/strategies/preview', { method: 'POST', body: JSON.stringify({ source: 'freezeScan', operationId: id, candidateIndex: resultIndex }) }),
  applyStrategy: (source: 'strategyPick' | 'freezeScan', operationId: string, candidateIndex: number, expectedRevision: string) => api<{ config: ConfigResponse; message: string }>('/api/nfqws2/strategies/apply', { method: 'POST', body: JSON.stringify({ source, operationId, candidateIndex, expectedRevision }) }),
};

export const nfqwsUpdates = {
  uiStatus: () => api<UiUpdateStatus>('/api/updates/status'),
  releases: () => api<{ releases: PanelRelease[] }>('/api/updates/releases'),
  install: (tag: string) => api<UpdateOperation>('/api/updates/install', {
    method: 'POST', body: JSON.stringify({ tag }),
  }),
  status: (id: string) => api<UpdateOperation>(`/api/updates/operations/${encodeURIComponent(id)}`),
  coreReleases: () => api<NfqwsCoreReleases>('/api/nfqws2/updates/releases'),
  installCore: (tag: string) => api<UpdateOperation>('/api/nfqws2/updates/install', {
    method: 'POST', body: JSON.stringify({ tag }),
  }),
};
