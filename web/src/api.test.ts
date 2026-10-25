import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, nfqwsApply, nfqwsAppearance, nfqwsBackups, nfqwsConfig, nfqwsDiagnostics, nfqwsInstallation, nfqwsLists, nfqwsRuntime, nfqwsUpdates } from './api';

describe('API client used by the source-based panel', () => {
  it('reports the beginning of a non-JSON success response instead of silently returning an empty object', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('<html>upstream gateway failed</html>', { status: 200 })));
    const error = await api('/api/status').catch((value: unknown) => value);
    expect(error).toMatchObject({ name: 'ApiError', status: 200 });
    expect(error).toHaveProperty('message', expect.stringContaining('Сервер вернул не JSON'));
    expect(error).toHaveProperty('message', expect.stringContaining('upstream gateway failed'));
  });

  it('keeps plain-text error response details when the API fails outside JSON', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('PHP Fatal error: missing module', { status: 500 })));
    await expect(api('/api/status')).rejects.toMatchObject({
      name: 'ApiError',
      status: 500,
      message: expect.stringContaining('PHP Fatal error: missing module'),
    });
  });

  it('persists the selected appearance layout and palette with the standard status bar', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response('{"saved":true}', { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    const settings = { theme: 'light', layout: 'tiles', accent: 'cyan', statusBar: 'stacked', remindWindowsTcpTimestamps: true } as const;
    await nfqwsAppearance.save(settings);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/ui/appearance');
    expect(JSON.parse(String((fetchMock.mock.calls[0]?.[1] as RequestInit).body))).toEqual(settings);
  });

  it('starts, polls and cancels a persisted network freeze baseline scan', async () => {
    const operation = { id: '0123456789abcdef0123456789abcdef', state: 'queued', stage: 'queued', current: null, done: 0, total: 20, results: [], message: 'queued', updatedAt: 100 };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(operation), { status: 202 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...operation, state: 'running', stage: 'baseline', current: 'Hetzner' }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...operation, state: 'stopping', stage: 'cleanup' }), { status: 202 }));
    vi.stubGlobal('fetch', fetchMock);
    const started = await nfqwsDiagnostics.freezeScanStart(['example.com']);
    expect(await nfqwsDiagnostics.freezeScanStatus(started.id)).toMatchObject({ state: 'running', current: 'Hetzner' });
    expect(await nfqwsDiagnostics.freezeScanCancel(started.id)).toMatchObject({ state: 'stopping' });
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/diagnostics/freeze-scan');
    expect(JSON.parse(String((fetchMock.mock.calls[0]?.[1] as RequestInit).body))).toEqual({ extraHosts: ['example.com'] });
    expect(fetchMock.mock.calls[1]?.[0]).toBe(`/api/nfqws2/diagnostics/freeze-scan/operations/${started.id}`);
    expect(fetchMock.mock.calls[2]?.[0]).toBe(`/api/nfqws2/diagnostics/freeze-scan/operations/${started.id}/cancel`);
  });

  it('starts, polls and stops a background router-side site trace through Rust', async () => {
    const operation = { id: '0123456789abcdef0123456789abcdef', state: 'queued', stage: 'queued', message: 'Трассировка поставлена в очередь', report: null, updatedAt: 100 };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(operation), { status: 202 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...operation, state: 'running', stage: 'router_probe' }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...operation, state: 'stopping', stage: 'cleanup' }), { status: 202 }));
    vi.stubGlobal('fetch', fetchMock);
    const started = await nfqwsDiagnostics.traceStart('example.com', 'https');
    expect(await nfqwsDiagnostics.traceStatus(started.id)).toMatchObject({ state: 'running', stage: 'router_probe' });
    expect(await nfqwsDiagnostics.traceStop(started.id)).toMatchObject({ state: 'stopping', stage: 'cleanup' });
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/diagnostics/trace');
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect(init.method).toBe('POST');
    expect(JSON.parse(String(init.body))).toEqual({ host: 'example.com', proto: 'https' });
    expect(fetchMock.mock.calls[1]?.[0]).toBe(`/api/nfqws2/diagnostics/trace/operations/${started.id}`);
    expect(fetchMock.mock.calls[2]?.[0]).toBe(`/api/nfqws2/diagnostics/trace/operations/${started.id}/stop`);
    expect((fetchMock.mock.calls[2]?.[1] as RequestInit).method).toBe('POST');
  });

  it('starts and polls a persisted isolated strategy-pick operation', async () => {
    const queued = { id: '0123456789abcdef0123456789abcdef', host: 'example.com', protocol: 'https', state: 'queued', stage: 'queued', current: null, done: 0, total: 3, results: [], message: 'queued', updatedAt: 100 } as const;
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(queued), { status: 202 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...queued, state: 'running', stage: 'testing', current: 'multisplit' }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...queued, state: 'stopping', stage: 'cleanup' }), { status: 202 }));
    vi.stubGlobal('fetch', fetchMock);
    const started = await nfqwsDiagnostics.strategyStart('example.com', 'https', 'custom', 4, [['--lua-desync=multisplit:pos=1']], true, true, true, true);
    expect(await nfqwsDiagnostics.strategyStatus(started.id)).toMatchObject({ state: 'running', current: 'multisplit' });
    expect(await nfqwsDiagnostics.strategyCancel(started.id)).toMatchObject({ state: 'stopping' });
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/strategies/pick');
    expect(JSON.parse(String((fetchMock.mock.calls[0]?.[1] as RequestInit).body))).toEqual({ host: 'example.com', protocol: 'https', mode: 'custom', repeats: 4, stopAfterFirstConfirmed: true, includeCommunity: true, includeOtherHistory: true, refineLighter: true, customSteps: [['--lua-desync=multisplit:pos=1']] });
  });

  it('starts, polls, and cancels a foreign configuration comparison', async () => {
    const operation = { id: '0123456789abcdef0123456789abcdef', state: 'queued', stage: 'queued', current: null, done: 0, total: 4, hosts: ['example.com'], comparisons: [], message: 'queued', updatedAt: 100 };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(operation), { status: 202 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...operation, state: 'running', current: 'Конфиг · example.com' }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ ...operation, state: 'stopping' }), { status: 202 }));
    vi.stubGlobal('fetch', fetchMock);
    const configs = [{ name: 'из чата', source: 'Текст', content: 'NFQWS_ARGS="--filter-tcp=443"' }, { name: 'по ссылке', source: 'Ссылка', url: 'https://raw.githubusercontent.com/user/repo/main/nfqws2.conf' }];
    const started = await nfqwsDiagnostics.foreignConfigStart(configs, ['example.com']);
    expect(await nfqwsDiagnostics.foreignConfigStatus(started.id)).toMatchObject({ state: 'running', current: 'Конфиг · example.com' });
    expect(await nfqwsDiagnostics.foreignConfigCancel(started.id)).toMatchObject({ state: 'stopping' });
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/diagnostics/config-check');
    expect(JSON.parse(String((fetchMock.mock.calls[0]?.[1] as RequestInit).body))).toEqual({ configs, hosts: ['example.com'] });
    expect(fetchMock.mock.calls[1]?.[0]).toBe(`/api/nfqws2/diagnostics/config-check/operations/${started.id}`);
    expect(fetchMock.mock.calls[2]?.[0]).toBe(`/api/nfqws2/diagnostics/config-check/operations/${started.id}/cancel`);
  });

  it('requests the full blockcheck2 candidate catalog explicitly', async () => {
    const queued = { id: '0123456789abcdef0123456789abcdef', host: 'example.com', protocol: 'https', state: 'queued', stage: 'queued', current: null, done: 0, total: 3100, results: [], message: 'queued', updatedAt: 100 } as const;
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(queued), { status: 202 }));
    vi.stubGlobal('fetch', fetchMock);
    await nfqwsDiagnostics.strategyStart('example.com', 'https', 'full');
    expect(JSON.parse(String((fetchMock.mock.calls[0]?.[1] as RequestInit).body))).toMatchObject({ host: 'example.com', protocol: 'https', mode: 'full' });
  });

  it('loads persisted strategy-pick history from Rust', async () => {
    const history = [{ id: '0123456789abcdef0123456789abcdef', host: 'example.com', protocol: 'https', state: 'complete', stage: 'complete', current: null, done: 2, total: 2, results: [], message: 'complete', updatedAt: 100 }];
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(history), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    expect(await nfqwsDiagnostics.strategyHistory()).toEqual(history);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/strategies/pick/history');
  });

  it('requests a backend-generated strategy preview by persisted operation and candidate index', async () => {
    const preview = { operationId: '0123456789abcdef0123456789abcdef', candidateIndex: 0, host: 'example.com', protocol: 'https', candidate: { name: 'multisplit', source: 'current', steps: ['--lua-desync=multisplit:pos=1'] }, originalRevision: 'abc', originalContent: 'NFQWS_ARGS_CUSTOM=""\n', proposedContent: 'NFQWS_ARGS_CUSTOM="--new --filter-tcp=443"\n', summary: 'Preview only' };
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(preview), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    expect(await nfqwsDiagnostics.strategyPreview(preview.operationId, 0)).toEqual(preview);
    expect(fetchMock.mock.calls[0]?.[0]).toBe(`/api/nfqws2/strategies/pick/operations/${preview.operationId}/preview`);
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect(init.method).toBe('POST');
    expect(JSON.parse(String(init.body))).toEqual({ candidateIndex: 0 });
  });

  it('previews and applies only a server-confirmed strategy with an expected config revision', async () => {
    const preview = { operationId: '0123456789abcdef0123456789abcdef', candidateIndex: 3, host: 'example.com', protocol: 'https', candidate: { name: 'freeze', source: 'scan', steps: ['--lua-desync=hostfakesplit:host=ya.ru'] }, originalRevision: 'rev-1', originalContent: '', proposedContent: '', summary: 'preview' };
    const response = { config: { file: '/opt/etc/nfqws2.conf', content: 'new', revision: 'rev-2' }, message: 'restarted' };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(preview), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(response), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    expect(await nfqwsDiagnostics.freezeStrategyPreview(preview.operationId, 3)).toEqual(preview);
    expect(JSON.parse(String(fetchMock.mock.calls[0]?.[1]?.body))).toEqual({ source: 'freezeScan', operationId: preview.operationId, candidateIndex: 3 });
    expect(await nfqwsDiagnostics.applyStrategy('freezeScan', preview.operationId, 3, preview.originalRevision)).toEqual(response);
    expect(fetchMock.mock.calls[1]?.[0]).toBe('/api/nfqws2/strategies/apply');
    expect(JSON.parse(String(fetchMock.mock.calls[1]?.[1]?.body))).toEqual({ source: 'freezeScan', operationId: preview.operationId, candidateIndex: 3, expectedRevision: 'rev-1' });
  });

  it('requests router-side website diagnostics and returns the real stage results', async () => {
    const report = { host: 'example.com', addresses: ['1.1.1.1'], steps: [{id:'https',status:'error',detail:'TLS timeout',durationMs:8000,httpCode:null}], summary: 'Не удалось проверить HTTPS', limitations: [] };
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(report), {status:200}));
    vi.stubGlobal('fetch', fetchMock);
    expect(await nfqwsDiagnostics.site('example.com')).toEqual(report);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/diagnostics/site');
    expect(JSON.parse(String(fetchMock.mock.calls[0]?.[1].body))).toEqual({host:'example.com'});
  });

  it('previews a quick setting through Rust without saving the draft', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response('{"content":"ISP_INTERFACE=eth3\\n","changedVariable":"ISP_INTERFACE"}', { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    const result = await nfqwsConfig.preview('ISP_INTERFACE=wan\n', 'ISP_INTERFACE', 'eth3');
    expect(result.content).toBe('ISP_INTERFACE=eth3\n');
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/config/preview');
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect(init.method).toBe('POST');
    expect(JSON.parse(String(init.body))).toEqual({content: 'ISP_INTERFACE=wan\n', key: 'ISP_INTERFACE', value: 'eth3'});
  });

  it('validates config drafts through the Rust backend and reports check limitations', async () => {
    const validation = { valid: true, diagnostics: [{ severity: 'warning', code: 'dynamic_shell_value', message: 'Не раскрывается', line: 4 }], checks: { parser: 'passed', installedBinary: 'not_run' } } as const;
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(validation), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    expect(await nfqwsConfig.validate('NFQWS_EXTRA_ARGS="$MODE_AUTO"\n')).toEqual(validation);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/config/validate');
    expect(JSON.parse(String((fetchMock.mock.calls[0]?.[1] as RequestInit).body))).toEqual({ content: 'NFQWS_EXTRA_ARGS="$MODE_AUTO"\n' });
  });

  it('formats a config draft through the Rust backend without applying it', async () => {
    const formatted = { content: 'NFQWS_ARGS="--filter-tcp=443\n    --lua-desync=fake"\n' };
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(formatted), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    expect(await nfqwsConfig.format('NFQWS_ARGS="--filter-tcp=443 --lua-desync=fake"\n')).toEqual(formatted);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/config/format');
    expect((fetchMock.mock.calls[0]?.[1] as RequestInit).method).toBe('POST');
    expect(JSON.parse(String((fetchMock.mock.calls[0]?.[1] as RequestInit).body))).toEqual({
      content: 'NFQWS_ARGS="--filter-tcp=443 --lua-desync=fake"\n',
    });
  });

  afterEach(() => { vi.restoreAllMocks(); });

  it('sends same-origin requests without session or CSRF requirements', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response('{"ok":true}', { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    await api('/api/service', { method: 'POST', body: JSON.stringify({ action: 'restart' }) });

    const [, init] = fetchMock.mock.calls[0] as [RequestInfo, RequestInit];
    expect(init.credentials).toBe('same-origin');
    expect(new Headers(init.headers).has('X-CSRF-Token')).toBe(false);
  });

  it('surfaces backend errors as typed API errors', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('{"error":"Backend недоступен"}', { status: 503 })));
    await expect(api('/api/status')).rejects.toMatchObject({ status: 503, message: 'Backend недоступен' });
  });

  it('loads installation diagnostics through the typed API layer', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(
      '{"platform":"entware","config":{"path":"/opt/etc/nfqws2/nfqws2.conf","exists":true,"readable":true,"writable":true,"usedByActiveConfig":true},"configCandidates":[],"initScript":null,"initScriptCandidates":[],"binaryCandidates":[],"lists":[]}',
      { status: 200 },
    ));
    vi.stubGlobal('fetch', fetchMock);
    const result = await nfqwsInstallation.get();

    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/installation');
    expect(result.config?.usedByActiveConfig).toBe(true);
    expect(result.config?.writable).toBe(true);
  });

  it('loads the actual nfqws2 process profile inventory from Rust', async () => {
    const runtime = { running: true, pid: 81, source: 'proc-cmdline', profiles: [{index:1,name:null,filters:['--filter-tcp=443'],strategies:['fake'],payloads:['tls_client_hello']}], limitations: [] } as const;
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(runtime), {status:200}));
    vi.stubGlobal('fetch', fetchMock);
    expect(await nfqwsRuntime.get()).toEqual(runtime);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/runtime');
  });

  it('saves a selected list using its server revision without a session token', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(
      '{"id":"user.list","path":"/opt/etc/nfqws2/lists/user.list","content":"example.org\\n","revision":"next"}',
      { status: 200 },
    ));
    vi.stubGlobal('fetch', fetchMock);
    const result = await nfqwsLists.save('user.list', 'example.org\n', 'current');

    expect(result.revision).toBe('next');
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/lists/user.list');
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect(init.method).toBe('PUT');
    expect(new Headers(init.headers).has('X-CSRF-Token')).toBe(false);
    expect(JSON.parse(String(init.body))).toEqual({ content: 'example.org\n', revision: 'current' });
  });

  it('creates and restores snapshots through the backend API', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response('{"id":"nfqws2.conf.123.000000001.bak","createdAt":123,"file":"nfqws2.conf"}', { status: 201 }))
      .mockResolvedValueOnce(new Response('{"message":"Конфигурация восстановлена"}', { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);

    const backup = await nfqwsBackups.create();
    const restored = await nfqwsBackups.restore(backup.id);

    expect(backup.file).toBe('nfqws2.conf');
    expect(restored.message).toContain('восстановлена');
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/backups');
    expect((fetchMock.mock.calls[0]?.[1] as RequestInit).method).toBe('POST');
    expect(fetchMock.mock.calls[1]?.[0]).toBe('/api/backups/nfqws2.conf.123.000000001.bak/restore');
  });

  it('applies a config through the transactional endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(
      '{"config":{"file":"/opt/etc/nfqws2/nfqws2.conf","content":"NFQWS_ARGS=--filter-tcp=443\\n","revision":"new"},"message":"Started and verified"}',
      { status: 200 },
    ));
    vi.stubGlobal('fetch', fetchMock);
    const result = await nfqwsApply.apply('NFQWS_ARGS=--filter-tcp=443\n', 'old');

    expect(result.config.revision).toBe('new');
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/apply');
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect(init.method).toBe('POST');
    expect(JSON.parse(String(init.body))).toEqual({ content: 'NFQWS_ARGS=--filter-tcp=443\n', revision: 'old' });
  });

  it('lists real eligible releases and starts a selected-version update through Rust', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response('{"releases":[{"tag":"v0.1.7","publishedAt":"2026-10-22T00:00:00Z","notes":"notes"}],"updateStatus":{"currentVersion":"0.1.6","latestVersion":"v0.1.7","updateAvailable":true,"releaseUrl":"https://github.com/rangerxxyy/nfqws2-ui/releases/tag/v0.1.7","checkedAt":123}}', { status: 200 }))
      .mockResolvedValueOnce(new Response('{"id":"abc123","state":"downloading"}', { status: 202 }));
    vi.stubGlobal('fetch', fetchMock);

    const releases = await nfqwsUpdates.releases();
    const operation = await nfqwsUpdates.install('v0.1.7');

    expect(releases.releases[0]?.tag).toBe('v0.1.7');
    expect(releases.updateStatus).toMatchObject({ updateAvailable: true, latestVersion: 'v0.1.7' });
    expect(operation.state).toBe('downloading');
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/updates/releases');
    expect(fetchMock.mock.calls[1]?.[0]).toBe('/api/updates/install');
    expect(JSON.parse(String((fetchMock.mock.calls[1]?.[1] as RequestInit).body))).toEqual({ tag: 'v0.1.7' });
  });

  it('loads the cached background UI update status from the local backend', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(
      '{"currentVersion":"0.3.4","latestVersion":"v0.3.5","updateAvailable":true,"releaseUrl":"https://github.com/rangerxxyy/nfqws2-ui/releases/tag/v0.3.5","checkedAt":123}',
      { status: 200 },
    ));
    vi.stubGlobal('fetch', fetchMock);

    const status = await nfqwsUpdates.uiStatus();

    expect(status.updateAvailable).toBe(true);
    expect(status.latestVersion).toBe('v0.3.5');
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/updates/status');
  });

  it('polls update status by operation id', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response('{"id":"abc123","state":"success","message":"Установлено"}', { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);

    const operation = await nfqwsUpdates.status('abc123');

    expect(operation.state).toBe('success');
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/updates/operations/abc123');
  });

  it('loads nfqws2 core releases and requests a core package update through Rust', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response('{"releases":[{"tag":"v1.3.2","publishedAt":"2026-10-21T11:04:30Z","notes":"Core release"}],"installedTag":"v1.3.1"}', { status: 200 }))
      .mockResolvedValueOnce(new Response('{"id":"abc123","state":"downloading"}', { status: 202 }));
    vi.stubGlobal('fetch', fetchMock);

    const catalog = await nfqwsUpdates.coreReleases();
    const operation = await nfqwsUpdates.installCore('v1.3.2');

    expect(catalog.installedTag).toBe('v1.3.1');
    expect(catalog.releases[0]?.tag).toBe('v1.3.2');
    expect(operation.state).toBe('downloading');
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/updates/releases');
    expect(fetchMock.mock.calls[1]?.[0]).toBe('/api/nfqws2/updates/install');
    expect(JSON.parse(String((fetchMock.mock.calls[1]?.[1] as RequestInit).body))).toEqual({ tag: 'v1.3.2' });
  });
});
