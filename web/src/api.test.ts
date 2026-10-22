import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, nfqwsApply, nfqwsBackups, nfqwsInstallation, nfqwsLists, nfqwsUpdates } from './api';

describe('API client used by the source-based panel', () => {
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
      .mockResolvedValueOnce(new Response('{"releases":[{"tag":"v0.1.7","publishedAt":"2026-10-22T00:00:00Z","notes":"notes"}]}', { status: 200 }))
      .mockResolvedValueOnce(new Response('{"id":"abc123","state":"downloading"}', { status: 202 }));
    vi.stubGlobal('fetch', fetchMock);

    const releases = await nfqwsUpdates.releases();
    const operation = await nfqwsUpdates.install('v0.1.7');

    expect(releases.releases[0]?.tag).toBe('v0.1.7');
    expect(operation.state).toBe('downloading');
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/updates/releases');
    expect(fetchMock.mock.calls[1]?.[0]).toBe('/api/updates/install');
    expect(JSON.parse(String((fetchMock.mock.calls[1]?.[1] as RequestInit).body))).toEqual({ tag: 'v0.1.7' });
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
