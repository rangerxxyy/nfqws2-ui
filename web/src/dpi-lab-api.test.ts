import { afterEach, describe, expect, it, vi } from 'vitest';
import { nfqwsDpiLab } from './api';

afterEach(() => vi.unstubAllGlobals());

describe('DPI Lab API', () => {
  it('sends strategy source to the bounded Rust analyzer as JSON', async () => {
    const source = '--lua-desync=multisplit:pos=midsld';
    const fetchMock = vi.fn().mockResolvedValue(new Response('{"stepCount":1,"steps":[]}', { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    await nfqwsDpiLab.analyzeStrategy(source);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/dpi-lab/strategy/analyze');
    expect(new Headers((fetchMock.mock.calls[0]?.[1] as RequestInit).headers).get('Content-Type')).toBe('application/json');
    expect(JSON.parse((fetchMock.mock.calls[0]?.[1] as RequestInit).body as string)).toEqual({ source });
  });

  it('uploads the selected PCAP as bounded binary data with an encoded display name', async () => {
    const file = new File([new Uint8Array([1, 2, 3])], 'capture sample.pcap');
    const fetchMock = vi.fn().mockResolvedValue(new Response('{"packets":[],"flows":[]}', { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    await nfqwsDpiLab.analyze(file);
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/nfqws2/dpi-lab/analyze?name=capture+sample.pcap');
    expect(new Headers((fetchMock.mock.calls[0]?.[1] as RequestInit).headers).get('Content-Type')).toBe('application/octet-stream');
    expect((fetchMock.mock.calls[0]?.[1] as RequestInit).body).toBe(file);
  });
});
