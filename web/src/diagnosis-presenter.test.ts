import { describe, expect, it } from 'vitest';
import { comparisonVerdict, presentStep, presentTrace, quicExplanation } from './diagnosis-presenter';
import type { SiteDiagnosticStep, SiteTracePhase } from './api';
const step = (status: SiteDiagnosticStep['status'], httpCode: number | null = null): SiteDiagnosticStep => ({ id: 'https', status, httpCode, durationMs: null, detail: '' });
describe('diagnosis presentation', () => {
  it.each(['ok', 'warning', 'error', 'unavailable'] as const)('does not call %s unavailable checks blocked', status => {
    if (status === 'unavailable' || status === 'warning') expect(presentStep(step(status)).kind).toBe('unknown');
    else expect(presentStep(step(status)).kind).not.toBe('unknown');
  });
  it.each([403, 404, 500])('describes HTTP %s as a server error even with successful transport', code => {
    expect(presentStep(step('ok', code)).text).toContain(`Сервер вернул HTTP ${code}`);
    expect(presentTrace({ state: 'complete', ok: true, code, durationMs: null, reason: '' }, 'current').text).toContain(`Сервер вернул HTTP ${code}`);
  });
  it.each([['ok', 'ok', 'success'], ['error', 'ok', 'success'], ['ok', 'error', 'failure'], ['error', 'error', 'failure'], ['unavailable', 'ok', 'unknown'], ['ok', 'unavailable', 'unknown'], ['warning', 'ok', 'unknown']] as const)('compares %s / %s', (direct, current, kind) => {
    expect(comparisonVerdict(step(direct), step(current)).kind).toBe(kind);
  });
  it('covers every direct/current status pair without turning uncertain checks into blockage', () => {
    const statuses = ['ok', 'warning', 'error', 'unavailable'] as const;
    for (const direct of statuses) for (const current of statuses) {
      const result = comparisonVerdict(step(direct), step(current));
      const uncertain = direct === 'warning' || direct === 'unavailable' || current === 'warning' || current === 'unavailable';
      expect(result.kind).toBe(uncertain ? 'unknown' : current === 'ok' ? 'success' : 'failure');
    }
  });
  it.each(['direct', 'current'] as const)('distinguishes failed and unavailable trace %s', route => {
    const phase: SiteTracePhase = { state: 'error', ok: false, code: null, durationMs: null, reason: '' };
    expect(presentTrace(phase, route).text).toBe(route === 'direct' ? 'Не открылся напрямую' : 'Не открылся через текущую стратегию nfqws2');
    expect(presentTrace({ ...phase, state: 'unavailable' }, route).kind).toBe('unknown');
    expect(presentTrace({ ...phase, ok: null }, route).kind).toBe('unknown');
  });
  it('keeps QUIC network response distinct from HTTP/3 and advertising', () => {
    expect(quicExplanation('quic_initial', true)).toContain('не полный запрос HTTP/3');
    expect(quicExplanation('quic_initial', true)).toContain('объявил');
    expect(quicExplanation('http3', null)).toContain('Полноценный запрос HTTP/3');
  });
});
