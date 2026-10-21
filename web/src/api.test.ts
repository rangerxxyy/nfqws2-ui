import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { api, setCsrfToken } from './api';

describe('API client used by the source-based panel', () => {
  beforeEach(() => { setCsrfToken(''); });
  afterEach(() => { vi.restoreAllMocks(); });

  it('sends same-origin credentials and CSRF on mutating calls', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response('{"ok":true}', { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    setCsrfToken('csrf-test-token');

    await api('/api/service', { method: 'POST', body: JSON.stringify({ action: 'restart' }) });

    const [, init] = fetchMock.mock.calls[0] as [RequestInfo, RequestInit];
    expect(init.credentials).toBe('same-origin');
    expect(new Headers(init.headers).get('X-CSRF-Token')).toBe('csrf-test-token');
  });

  it('surfaces backend errors as typed API errors', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('{"error":"Требуется вход"}', { status: 401 })));
    await expect(api('/api/status')).rejects.toMatchObject({ status: 401, message: 'Требуется вход' });
  });
});
