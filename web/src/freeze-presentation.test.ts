import { describe, expect, it } from 'vitest';
import { freezeMeasurementDetails, freezeRowPresentation, freezeCanApplyToHost } from './freeze-presentation';

describe('freeze scan row presentation', () => {
  it('keeps direct and nfqws2 measurements available in one compact disclosure', () => {
    expect(freezeMeasurementDetails('ответ получен · 84 мс', 'ответ получен · 83 мс'))
      .toBe('Напрямую: ответ получен · 84 мс\nЧерез nfqws2: ответ получен · 83 мс');
  });

  it('marks possible freeze findings red without claiming they are proven', () => {
    expect(freezeRowPresentation({ freezeDetected: true, foundStrategy: null, directOk: false, currentOk: false })).toEqual({ tone: 'error', label: 'Возможный обрыв' });
    expect(freezeRowPresentation({ freezeDetected: false, foundStrategy: null, directOk: true, currentOk: true })).toEqual({ tone: 'success', label: 'Ответ получен' });
    expect(freezeRowPresentation({ freezeDetected: true, foundStrategy: {}, directOk: false, currentOk: false }).tone).toBe('success');
    expect(freezeRowPresentation({ freezeDetected: true, foundStrategy: { reusedFrom: 'hetzner.com' }, directOk: false, currentOk: false }).tone).toBe('warning');
  });

  it('keeps inconclusive and pending results neutral', () => {
    expect(freezeRowPresentation({ freezeDetected: false, foundStrategy: null, directOk: false, currentOk: true }).tone).toBe('neutral');
  });

  it('allows applying only a confirmed strategy from an explicit user-host check', () => {
    expect(freezeCanApplyToHost({ state: 'complete', targetKind: 'user_host', freezeDetected: true, foundStrategy: { attempts: 3, successes: 3 } })).toBe(true);
    expect(freezeCanApplyToHost({ state: 'running', targetKind: 'user_host', freezeDetected: true, foundStrategy: { attempts: 3, successes: 3 } })).toBe(true);
    expect(freezeCanApplyToHost({ state: 'partial', targetKind: 'user_host', freezeDetected: true, foundStrategy: { attempts: 3, successes: 3 } })).toBe(true);
    expect(freezeCanApplyToHost({ state: 'complete', targetKind: 'user_host', freezeDetected: false, foundStrategy: { attempts: 3, successes: 3 } })).toBe(true);
    expect(freezeCanApplyToHost({ state: 'complete', targetKind: 'user_host', freezeDetected: true, foundStrategy: { attempts: 3, successes: 3, reusedFrom: 'other.example' } })).toBe(false);
    expect(freezeCanApplyToHost({ state: 'complete', targetKind: 'pinned_address', freezeDetected: true, foundStrategy: { attempts: 3, successes: 3 } })).toBe(false);
    expect(freezeCanApplyToHost({ state: 'partial', targetKind: 'user_host', freezeDetected: true, foundStrategy: { attempts: 3, successes: 2 } })).toBe(false);
  });
});
