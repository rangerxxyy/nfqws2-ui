import { describe, expect, it } from 'vitest';
import { appendMonitoringResult, checkMonitoringHostsSequentially, loadMonitoringState, normalizeMonitoringHost, saveMonitoringState } from './monitoring';

describe('monitoring state', () => {
  it('accepts hostnames and rejects URLs, IP literals, and malformed input', () => {
    expect(normalizeMonitoringHost(' Example.COM ')).toBe('example.com');
    expect(normalizeMonitoringHost('https://example.com')).toBeNull();
    expect(normalizeMonitoringHost('192.0.2.1')).toBeNull();
    expect(normalizeMonitoringHost('bad host')).toBeNull();
  });

  it('bounds sites and recent result history and tolerates unavailable storage', () => {
    const state = loadMonitoringState(null);
    for (let i = 0; i < 25; i++) state.sites.push({ host: `site${i}.example`, results: [] });
    const report = { host: 'site0.example', addresses: [], steps: [], summary: 'ok', limitations: [], direct: { id: 'direct', status: 'ok' as const, detail: '', durationMs: 12, httpCode: 200 }, current: { id: 'current', status: 'ok' as const, detail: '', durationMs: 13, httpCode: 200 }, quic: null };
    let updated = state;
    for (let i = 0; i < 12; i++) updated = appendMonitoringResult(updated, 'site0.example', report, i);
    expect(updated.sites).toHaveLength(20);
    expect(updated.sites[0].results).toHaveLength(10);
    expect(updated.sites[0].results[0].checkedAt).toBe(11);
    expect(() => saveMonitoringState(null, updated)).not.toThrow();
  });

  it('checks hosts sequentially and continues after an individual failure', async () => {
    const calls: string[] = [];
    const failures: string[] = [];
    await checkMonitoringHostsSequentially(['a.example', 'b.example', 'c.example'], async (host) => {
      calls.push(`start:${host}`);
      await Promise.resolve();
      calls.push(`end:${host}`);
      if (host === 'b.example') throw new Error('offline');
      return { host, addresses: [], steps: [], summary: 'ok', limitations: [], direct: { id: 'direct', status: 'ok' as const, detail: '', durationMs: 1, httpCode: 200 }, current: { id: 'current', status: 'ok' as const, detail: '', durationMs: 1, httpCode: 200 }, quic: null };
    }, () => undefined, (host) => failures.push(host));
    expect(calls).toEqual(['start:a.example', 'end:a.example', 'start:b.example', 'end:b.example', 'start:c.example', 'end:c.example']);
    expect(failures).toEqual(['b.example']);
  });
});
