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
export type SiteDiagnosticReport = { host: string; addresses: string[]; reverseDns?: Array<{ address: string; names: string[]; resolver: string | null; detail: string }>; steps: SiteDiagnosticStep[]; summary: string; limitations: string[]; direct: SiteDiagnosticStep; current: SiteDiagnosticStep; quic: null | { direct: SiteDiagnosticStep; current: SiteDiagnosticStep; method: 'quic_initial' | 'http3'; http3Advertised: boolean | null } };
export type SiteTracePhase = { state: 'complete' | 'error' | 'unavailable'; ok: boolean | null; code: number | null; durationMs: number | null; diagnosticClassification?: string | null; reason: string };
export type SiteTraceReport = { host: string; address: string | null; withoutNfqws2: SiteTracePhase; withNfqws2: SiteTracePhase; traceLines: string[]; traceTruncated: boolean; traceHostname: string | null; traceProfile: string | null; traceActions: string[]; traceWarnings: string[]; summary: string };
export type SiteTraceOperation = { id: string; state: 'queued' | 'running' | 'stopping' | 'complete' | 'cancelled' | 'failed'; stage: string; message: string; report: SiteTraceReport | null; updatedAt: number };
export type StrategyPickResult = { candidate: { name: string; source: string; steps: string[] }; attempts: number; successes: number; confirmed: boolean; elapsedMs: number; detail: string };
export type MultiSiteBaselineResult = { host: string; eligible: boolean; reason: string };
export type MultiSiteDomainResult = { host: string; eligible: boolean; baselineReason: string; attempts: number; successes: number; elapsedMs: number; confirmed: boolean };
export type MultiSiteCandidateResult = { candidate: StrategyPickResult['candidate']; openedDomains: number; domains: MultiSiteDomainResult[] };
export type StrategyPickOperation = { id: string; host: string; hosts?: string[]; protocol: string; state: 'queued' | 'running' | 'stopping' | 'complete' | 'partial' | 'no_match' | 'site_unblocked' | 'cancelled' | 'failed'; stage: string; current: string | null; done: number; total: number; results: StrategyPickResult[]; multiSiteBaseline?: MultiSiteBaselineResult[]; multiSiteResults?: MultiSiteCandidateResult[]; message: string; updatedAt: number };
export type StrategyPickPreview = { operationId: string; candidateIndex: number; host: string; protocol: 'http' | 'https'; candidate: StrategyPickResult['candidate']; originalRevision: string; originalContent: string; proposedContent: string; summary: string };
export type FreezeStrategyMatch = { sni: string; step: string; attempts: number; successes: number; averageMs: number | null; reusedFrom: string | null };
export type FreezeCandidateAttempt = { sni: string; step: string; attempts: number; successes: number; averageMs: number | null; confirmed: boolean; reason: string | null };
export type FreezeNetworkResult = { label: string; host: string; path: string; targetKind: 'host_reference' | 'pinned_address' | 'user_host'; address: string | null; asnHint: number | null; asn: { state: 'verified'; asn: number } | { state: 'changed'; expected: number; actual: number } | { state: 'unavailable' }; direct: { classification: string; ok: boolean; bytesReceived: number | null; replies: number | null; elapsedMs: number; reason: string }; current: { classification: string; ok: boolean; bytesReceived: number | null; replies: number | null; elapsedMs: number; reason: string }; freezeDetected: boolean; triedCandidates: number; candidateTotal: number; foundStrategy: FreezeStrategyMatch | null; candidateAttempts: FreezeCandidateAttempt[] };
export type FreezeScanOperation = { id: string; state: 'queued' | 'running' | 'stopping' | 'complete' | 'partial' | 'cancelled' | 'failed'; stage: string; current: string | null; done: number; total: number; results: FreezeNetworkResult[]; message: string; updatedAt: number };
export type ForeignConfigCheckOperation = { id: string; state: 'queued' | 'running' | 'stopping' | 'complete' | 'cancelled' | 'failed'; stage: string; current: string | null; done: number; total: number; hosts: string[]; comparisons: { name: string; source: string; sites: { host: string; currentOk: boolean | null; currentDetail: string; currentAttempts: number; currentSuccesses: number; currentAverageMs: number | null; candidateOk: boolean | null; candidateDetail: string; candidateAttempts: number; candidateSuccesses: number; candidateAverageMs: number | null }[] }[]; message: string; updatedAt: number };
export type NfqwsRuntimeProfile = { index: number; name: string | null; filters: string[]; strategies: string[]; payloads: string[] };
export type NfqwsRuntime = { running: boolean; pid: number | null; source: 'proc-cmdline'; profiles: NfqwsRuntimeProfile[]; limitations: string[] };
export type NfqwsListId = string;
export type NfqwsListSummary = { id: NfqwsListId; path: string; readable: boolean; writable: boolean };
export type NfqwsListResponse = { id: NfqwsListId; path: string; content: string; revision: string };
export type MonitoringProbeStep = { id: string; status: 'ok' | 'warning' | 'error' | 'unavailable'; detail: string; durationMs: number | null; httpCode: number | null };
export type MonitoringProbeReport = { host: string; summary: string; current: { status: 'ok' | 'warning' | 'error' | 'unavailable'; detail: string; durationMs: number | null; httpCode: number | null }; steps: MonitoringProbeStep[] };
export type MonitoringSettings = { enabled: boolean; intervalMinutes: number; lastAutoCheck: number | null; sites: Array<{ host: string; results: Array<{ checkedAt: number; status: 'ok' | 'warning' | 'error' | 'unavailable'; summary: string; latencyMs: number | null; steps: MonitoringProbeStep[] }> }> };
export type TelegramQueuedMessage = { timestamp: number; text: string; tries: number; error?: string | null };
export type TelegramDeliveryLog = { timestamp: number; eventTimestamp: number; text: string; ok: boolean; error?: string | null };
export type TelegramSettings = { tokenSet: boolean; chatId: string; sendVia: 'direct' | 'interface' | 'proxy'; interface: string; proxy: string; notifications: { queue: TelegramQueuedMessage[]; log: TelegramDeliveryLog[] } };
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
export type NfqwsLuaScriptInfo = { name: string; path: string | null; exists: boolean; readable: boolean };
export type NfqwsLuaScriptContent = { name: string; path: string; compressed: boolean; content: string; truncated: boolean };
export type BackupItem = { id: string; createdAt: number; file: string };
export type SnapshotItem = { id: string; ts: number; reason: string; note: string; size: number; local: boolean; remote: boolean; pinned: boolean; files: Record<string, string> };
export type SnapshotRemoteSettings = { enabled: boolean; host: string; port: number; user: string; dir: string; keepLocal: number };
export type SnapshotSettings = { maxCount: number; maxDays: number; remote: SnapshotRemoteSettings };
export type SnapshotOverview = { items: SnapshotItem[]; settings: SnapshotSettings; directory: string; totalBytes: number; localCount: number; remote: { tool: string | null; publicKey: string; on: boolean; pending: number; stored: number; lastOk: number | null; error: string | null; setupCommand: string } };
export type NfqwsArgumentProfile = { id: string; index: number; name: string; source: string; editable: boolean; args: string[]; filters: string[]; protocols: string[]; lists: string[]; steps: string[] };
export type NfqwsProfilesResponse = { profiles: NfqwsArgumentProfile[]; customArgs: string; revision: string; limitation: string | null };
export type NfqwsLuaFunctionInfo = { name: string; file: string; docs: string[]; arguments: string[]; fileArguments?: string[]; standardGroups: string[]; required: string[] };
export type NfqwsLuaStandardGroup = { name: string; arguments: string[] };
export type NfqwsLuaCatalogResponse = { functions: NfqwsLuaFunctionInfo[]; standardGroups: NfqwsLuaStandardGroup[] };
export type ChangeHistoryEntry = { id: string; ts: number; version: string | null; reason: string; note: string; changedFiles: string[] };
export type LogSource = 'nfqws2-ui' | 'nfqws2';
export type UiAppearanceSettings = { theme: 'dark' | 'light' | 'neon'; layout: 'standard' | 'compact' | 'dense' | 'wide' | 'right' | 'top' | 'tiles' | 'simplified'; accent: 'blue' | 'cyan' | 'emerald' | 'amber' | 'rose' | 'violet'; statusBar: 'standard' | 'minimal' | 'compact' | 'stacked' | 'reverse'; remindWindowsTcpTimestamps: boolean };
export type PanelAuthStatus = { enabled: boolean; hasPassword: boolean; authenticated: boolean; rememberDays: number };
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

async function readApiResponse(response: Response): Promise<unknown> {
  const text = await response.text();
  let payload: unknown;
  try {
    payload = text.trim() ? JSON.parse(text) as unknown : {};
  } catch {
    const preview = text.replace(/\s+/g, ' ').trim().slice(0, 240);
    throw new ApiError(response.status, `Сервер вернул не JSON${preview ? `: ${preview}` : ': пустой ответ'}`);
  }

  if (!response.ok) {
    const message = payload && typeof payload === 'object' && 'error' in payload && typeof payload.error === 'string'
      ? payload.error
      : `HTTP ${response.status}`;
    throw new ApiError(response.status, message);
  }
  return payload;
}

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.body && !headers.has('Content-Type')) headers.set('Content-Type', 'application/json');
  const response = await fetch(path, { ...init, headers, credentials: 'same-origin' });
  if (response.status === 401 && !path.startsWith('/api/auth/') && typeof window !== 'undefined') {
    window.dispatchEvent(new Event('nfqws-auth-expired'));
  }
  return await readApiResponse(response) as T;
}

export const nfqwsLists = {
  index: () => api<{ files: NfqwsListSummary[] }>('/api/nfqws2/lists'),
  get: (id: NfqwsListId) => api<NfqwsListResponse>(`/api/nfqws2/lists/${encodeURIComponent(id)}`),
  save: (id: NfqwsListId, content: string, revision: string) => api<NfqwsListResponse>(
    `/api/nfqws2/lists/${encodeURIComponent(id)}`,
    { method: 'PUT', body: JSON.stringify({ content, revision }) },
  ),
  delete: (id: NfqwsListId, revision: string) => api<{ message: string }>(
    `/api/nfqws2/lists/${encodeURIComponent(id)}`,
    { method: 'DELETE', body: JSON.stringify({ revision }) },
  ),
};

export const nfqwsInstallation = {
  get: () => api<NfqwsInstallation>('/api/nfqws2/installation'),
};

export const nfqwsScripts = {
  list: () => api<NfqwsLuaScriptInfo[]>('/api/nfqws2/scripts'),
  get: (name: string) => api<NfqwsLuaScriptContent>(`/api/nfqws2/scripts/${encodeURIComponent(name)}`),
};

export const nfqwsRuntime = {
  get: () => api<NfqwsRuntime>('/api/nfqws2/runtime'),
};

export const nfqwsBackups = {
  list: () => api<{ backups: BackupItem[] }>('/api/backups'),
  create: () => api<BackupItem>('/api/backups', { method: 'POST' }),
  restore: (id: string) => api<{ message: string }>(`/api/backups/${encodeURIComponent(id)}/restore`, { method: 'POST' }),
};

export const nfqwsSnapshots = {
  get: () => api<SnapshotOverview>('/api/snapshots'),
  create: (note = '') => api<SnapshotItem>('/api/snapshots', { method: 'POST', body: JSON.stringify({ note }) }),
  saveSettings: (settings: SnapshotSettings) => api<{ saved: boolean }>('/api/snapshots/settings', { method: 'PUT', body: JSON.stringify(settings) }),
  pin: (id: string, pinned: boolean) => api<{ saved: boolean }>(`/api/snapshots/${encodeURIComponent(id)}/pin`, { method: 'POST', body: JSON.stringify({ pinned }) }),
  delete: (id: string) => api<{ deleted: boolean }>(`/api/snapshots/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  restore: (id: string) => api<{ changed: string[]; message: string }>(`/api/snapshots/${encodeURIComponent(id)}/restore`, { method: 'POST' }),
  restoreFile: async (file: File) => {
    const response = await fetch('/api/snapshots/restore-file', { method: 'POST', body: file, headers: { 'Content-Type': 'application/gzip' }, credentials: 'same-origin' });
    return await readApiResponse(response) as { message?: string; changed?: string[] };
  },
  createKey: () => api<{ publicKey: string }>('/api/snapshots/remote/key', { method: 'POST' }),
  testRemote: () => api<{ ok: boolean; count: number }>('/api/snapshots/remote/test', { method: 'POST' }),
  pushRemote: () => api<{ sent: number }>('/api/snapshots/remote/push', { method: 'POST' }),
};

export const nfqwsProfiles = {
  list: () => api<NfqwsProfilesResponse>('/api/profiles'),
  catalog: () => api<NfqwsLuaCatalogResponse>('/api/profiles/catalog'),
  saveCustom: (profiles: string[][], revision: string) => api<{ config: ConfigResponse; message: string }>('/api/profiles', { method: 'PUT', body: JSON.stringify({ profiles, revision }) }),
};

export const nfqwsChangeHistory = {
  list: () => api<{ items: ChangeHistoryEntry[] }>('/api/change-history'),
};

export const nfqwsLogs = {
  get: (source: LogSource, tail = 1000) => api<{ content: string }>(`/api/logs?source=${encodeURIComponent(source)}&tail=${tail}`),
  clear: (source: LogSource) => api<{ cleared: boolean }>('/api/logs/clear', { method: 'POST', body: JSON.stringify({ source }) }),
};

export const nfqwsAppearance = {
  get: () => api<UiAppearanceSettings | null>('/api/ui/appearance'),
  save: (settings: UiAppearanceSettings) => api<{ saved: boolean }>('/api/ui/appearance', { method: 'PUT', body: JSON.stringify(settings) }),
};

export const nfqwsAuth = {
  status: () => api<PanelAuthStatus>('/api/auth/status'),
  login: (password: string, remember: boolean) => api<{ success: boolean }>('/api/auth/login', { method: 'POST', body: JSON.stringify({ password, remember }) }),
  logout: () => api<{ success: boolean }>('/api/auth/logout', { method: 'POST' }),
  save: (enabled: boolean, password: string, currentPassword: string, rememberDays: number) => api<{ saved: boolean }>('/api/auth/settings', { method: 'PUT', body: JSON.stringify({ enabled, password, currentPassword, rememberDays }) }),
};

export const nfqwsApply = {
  apply: (content: string, revision: string) => api<{ config: ConfigResponse; message: string }>(
    '/api/nfqws2/apply',
    { method: 'POST', body: JSON.stringify({ content, revision }) },
  ),
};

export const nfqwsConfig = {
  format: (content: string) => api<{ content: string }>('/api/nfqws2/config/format', {
    method: 'POST', body: JSON.stringify({ content }),
  }),
  validate: (content: string) => api<ConfigValidationResponse>('/api/nfqws2/config/validate', {
    method: 'POST', body: JSON.stringify({ content }),
  }),
  preview: (content: string, key: string, value: string) => api<{ content: string; changedVariable: string }>(
    '/api/nfqws2/config/preview',
    { method: 'POST', body: JSON.stringify({ content, key, value }) },
  ),
};

export const nfqwsDiagnostics = {
  supportReport: (hideSiteNames: boolean) => api<{ text: string }>('/api/nfqws2/diagnostics/support-report', { method: 'POST', body: JSON.stringify({ hideSiteNames }) }),
  site: (host: string) => api<SiteDiagnosticReport>('/api/nfqws2/diagnostics/site', {
    method: 'POST', body: JSON.stringify({ host }),
  }),
  monitor: (hosts: string[]) => api<MonitoringProbeReport[]>('/api/nfqws2/diagnostics/monitor', {
    method: 'POST', body: JSON.stringify({ hosts }),
  }),
  traceStart: (host: string, proto: 'http' | 'https') => api<SiteTraceOperation>('/api/nfqws2/diagnostics/trace', {
    method: 'POST', body: JSON.stringify({ host, proto }),
  }),
  traceStatus: (id: string) => api<SiteTraceOperation>(`/api/nfqws2/diagnostics/trace/operations/${encodeURIComponent(id)}`),
  traceStop: (id: string) => api<SiteTraceOperation>(`/api/nfqws2/diagnostics/trace/operations/${encodeURIComponent(id)}/stop`, { method: 'POST' }),
  freezeScanStart: (extraHosts: string[]) => api<FreezeScanOperation>('/api/nfqws2/diagnostics/freeze-scan', { method: 'POST', body: JSON.stringify({ extraHosts }) }),
  freezeScanStatus: (id: string) => api<FreezeScanOperation>(`/api/nfqws2/diagnostics/freeze-scan/operations/${encodeURIComponent(id)}`),
  freezeScanCancel: (id: string) => api<FreezeScanOperation>(`/api/nfqws2/diagnostics/freeze-scan/operations/${encodeURIComponent(id)}/cancel`, { method: 'POST' }),
  strategyStart: (host: string, protocol: 'http' | 'https', mode: 'quick' | 'extended' | 'full' | 'custom', repeats = 3, customSteps?: string[][], stopAfterFirstConfirmed = false, includeOtherHistory = false, refineLighter = false) => api<StrategyPickOperation>('/api/nfqws2/strategies/pick', { method: 'POST', body: JSON.stringify({ host, protocol, mode, repeats, stopAfterFirstConfirmed, includeOtherHistory, refineLighter, ...(customSteps ? { customSteps } : {}) }) }),
  strategyMultiStart: (hosts: string[], protocol: 'http' | 'https', repeats = 3, includeOtherHistory = true) => api<StrategyPickOperation>('/api/nfqws2/strategies/pick/multi', { method: 'POST', body: JSON.stringify({ hosts, protocol, repeats, includeOtherHistory }) }),
  strategyStatus: (id: string) => api<StrategyPickOperation>(`/api/nfqws2/strategies/pick/operations/${encodeURIComponent(id)}`),
  strategyCancel: (id: string) => api<StrategyPickOperation>(`/api/nfqws2/strategies/pick/operations/${encodeURIComponent(id)}/cancel`, { method: 'POST' }),
  strategyHistory: () => api<StrategyPickOperation[]>('/api/nfqws2/strategies/pick/history'),
  clearStrategyHistory: () => api<{ removed: number }>('/api/nfqws2/strategies/pick/history', { method: 'DELETE' }),
  strategyPreview: (id: string, candidateIndex: number) => api<StrategyPickPreview>(`/api/nfqws2/strategies/pick/operations/${encodeURIComponent(id)}/preview`, { method: 'POST', body: JSON.stringify({ candidateIndex }) }),
  freezeStrategyPreview: (id: string, resultIndex: number) => api<StrategyPickPreview>('/api/nfqws2/strategies/preview', { method: 'POST', body: JSON.stringify({ source: 'freezeScan', operationId: id, candidateIndex: resultIndex }) }),
  applyStrategy: (source: 'strategyPick' | 'freezeScan', operationId: string, candidateIndex: number, expectedRevision: string) => api<{ config: ConfigResponse; message: string }>('/api/nfqws2/strategies/apply', { method: 'POST', body: JSON.stringify({ source, operationId, candidateIndex, expectedRevision }) }),
  foreignConfigStart: (configs: { name: string; source: string; content?: string; url?: string }[], hosts: string[], repeats = 3) => api<ForeignConfigCheckOperation>('/api/nfqws2/diagnostics/config-check', { method: 'POST', body: JSON.stringify({ configs, hosts, repeats }) }),
  foreignConfigStatus: (id: string) => api<ForeignConfigCheckOperation>(`/api/nfqws2/diagnostics/config-check/operations/${encodeURIComponent(id)}`),
  foreignConfigCancel: (id: string) => api<ForeignConfigCheckOperation>(`/api/nfqws2/diagnostics/config-check/operations/${encodeURIComponent(id)}/cancel`, { method: 'POST' }),
  foreignConfigApply: (id: string, index: number) => api<{ config: ConfigResponse; message: string }>(`/api/nfqws2/diagnostics/config-check/operations/${encodeURIComponent(id)}/configs/${index}/apply`, { method: 'POST' }),
};

export const nfqwsMonitoring = {
  get: () => api<MonitoringSettings>('/api/nfqws2/monitoring'),
  save: (settings: Pick<MonitoringSettings, 'enabled' | 'intervalMinutes' | 'sites'>) => api<{ saved: boolean }>('/api/nfqws2/monitoring', { method: 'PUT', body: JSON.stringify(settings) }),
  run: (hosts?: string[]) => api<MonitoringProbeReport[]>('/api/nfqws2/monitoring/run', { method: 'POST', body: JSON.stringify(hosts ? { hosts } : {}) }),
  telegram: () => api<TelegramSettings>('/api/nfqws2/telegram'),
  telegramNotifications: () => api<{ queue: TelegramQueuedMessage[]; log: TelegramDeliveryLog[] }>('/api/nfqws2/telegram/notifications'),
  saveTelegram: (settings: { token?: string; chatId: string; sendVia: string; interface: string; proxy: string }) => api<{ saved: boolean; tokenSet: boolean }>('/api/nfqws2/telegram', { method: 'PUT', body: JSON.stringify(settings) }),
  testTelegram: () => api<{ sent: boolean }>('/api/nfqws2/telegram/test', { method: 'POST' }),
  findTelegramChats: () => api<{ chats: Array<{ id: string; title: string }> }>('/api/nfqws2/telegram/find-chats', { method: 'POST' }),
};

export const nfqwsUpdates = {
  uiStatus: () => api<UiUpdateStatus>('/api/updates/status'),
  releases: () => api<{ releases: PanelRelease[]; updateStatus: UiUpdateStatus }>('/api/updates/releases'),
  install: (tag: string) => api<UpdateOperation>('/api/updates/install', {
    method: 'POST', body: JSON.stringify({ tag }),
  }),
  status: (id: string) => api<UpdateOperation>(`/api/updates/operations/${encodeURIComponent(id)}`),
  coreReleases: () => api<NfqwsCoreReleases>('/api/nfqws2/updates/releases'),
  installCore: (tag: string) => api<UpdateOperation>('/api/nfqws2/updates/install', {
    method: 'POST', body: JSON.stringify({ tag }),
  }),
};
