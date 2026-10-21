import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, nfqwsInstallation, nfqwsLists } from './api';

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
});
