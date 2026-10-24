import { describe, expect, it } from 'vitest';
import { appendMonitoringResult, loadMonitoringState, normalizeMonitoringHost, saveMonitoringState } from './monitoring';

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
    const report = { host: 'site0.example', summary: 'ok', current: { status: 'ok' as const, detail: '', durationMs: 13, httpCode: 200 }, steps: [{ id: 'dns', status: 'ok' as const, detail: '8.8.8.8', durationMs: 2, httpCode: null }] };
    let updated = state;
    for (let i = 0; i < 12; i++) updated = appendMonitoringResult(updated, 'site0.example', report, i);
    expect(updated.sites).toHaveLength(20);
    expect(updated.sites[0].results).toHaveLength(10);
    expect(updated.sites[0].results[0].checkedAt).toBe(11);
    expect(updated.sites[0].results[0].steps[0].id).toBe('dns');
    expect(() => saveMonitoringState(null, updated)).not.toThrow();
  });

  it('keeps old version-1 monitoring history readable when it has no stage details', () => {
    const storage = { getItem: () => JSON.stringify({ version: 1, sites: [{ host: 'old.example', results: [{ checkedAt: 1, status: 'ok', summary: 'ok', latencyMs: 20 }] }] }) } as unknown as Storage;
    const state = loadMonitoringState(storage);
    expect(state.sites[0].results[0].summary).toBe('ok');
    expect(state.sites[0].results[0].steps).toEqual([]);
  });

});
