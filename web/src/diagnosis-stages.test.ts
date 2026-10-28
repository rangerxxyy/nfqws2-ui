import { describe, expect, it } from 'vitest';
import { diagnosisProgressState, diagnosisQueueMarker, isSkippedDiagnosisStep, SITE_DIAGNOSIS_STAGES } from './diagnosis-stages';

describe('site diagnosis stage queue', () => {
  it('lists checks in their intended order before a request starts', () => {
    expect(SITE_DIAGNOSIS_STAGES.map(({ id }) => id)).toEqual(['dns', 'tcp', 'direct', 'sni', 'http', 'current', 'quic']);
  });
  it('marks only results received from the backend complete and highlights the real current stage', () => {
    const completed = new Set(['dns']);
    expect(SITE_DIAGNOSIS_STAGES.map(({ id }) => diagnosisProgressState(id, 'tcp', completed))).toEqual(['complete', 'active', 'pending', 'pending', 'pending', 'pending', 'pending']);
    completed.add('tcp');
    expect(diagnosisProgressState('tcp', 'direct', completed)).toBe('complete');
    expect(diagnosisProgressState('direct', 'direct', completed)).toBe('active');
  });
  it('uses status icons instead of visible stage numbers', () => {
    expect(diagnosisQueueMarker('pending')).toBe('·');
    expect(diagnosisQueueMarker('active')).toBe('…');
    expect(diagnosisQueueMarker('complete', 'success')).toBe('✓');
    expect(diagnosisQueueMarker('complete', 'failure')).toBe('×');
  });
  it('recognizes checks intentionally skipped because they are not needed', () => {
    expect(isSkippedDiagnosisStep({ id: 'sni', status: 'unavailable', detail: 'Не нужно: сайт отвечает напрямую', durationMs: null, httpCode: null })).toBe(true);
    expect(isSkippedDiagnosisStep({ id: 'sni', status: 'unavailable', detail: 'Проверка недоступна: curl отсутствует', durationMs: null, httpCode: null })).toBe(false);
  });
});
