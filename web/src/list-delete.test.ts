import { afterEach, describe, expect, it, vi } from 'vitest';
import { nfqwsLists } from './api';

describe('custom list deletion API', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('deletes the requested custom list with DELETE', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ id: 'google.list', revision: 'rev1' }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ message: 'deleted' }), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);

    await nfqwsLists.delete('google.list', 'rev1');

    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/lists/google.list');
    expect((fetchMock.mock.calls[0]?.[1] as RequestInit).method).toBe('DELETE');
  });
});
