import { describe, expect, it } from 'vitest';
import { diagnosisQueueMarker, diagnosisQueueState, nextDiagnosisQueueIndex, SITE_DIAGNOSIS_STAGES } from './diagnosis-stages';

describe('site diagnosis stage queue', () => {
  it('lists checks in their intended order before a request starts', () => {
    expect(SITE_DIAGNOSIS_STAGES.map(({ id }) => id)).toEqual(['dns', 'tcp', 'direct', 'sni', 'http', 'current', 'quic']);
  });
  it('marks earlier steps complete, one active, and later steps pending', () => {
    expect(SITE_DIAGNOSIS_STAGES.map((_, index) => diagnosisQueueState(index, 2))).toEqual(['complete', 'complete', 'active', 'pending', 'pending', 'pending', 'pending']);
  });
  it('does not call earlier steps complete until the router returns results', () => {
    expect(SITE_DIAGNOSIS_STAGES.map((_, index) => diagnosisQueueState(index, 2, false))).toEqual(['pending', 'pending', 'active', 'pending', 'pending', 'pending', 'pending']);
  });
  it('advances the live highlight in order and stops at the last stage', () => {
    expect(nextDiagnosisQueueIndex(0, SITE_DIAGNOSIS_STAGES.length)).toBe(1);
    expect(nextDiagnosisQueueIndex(5, SITE_DIAGNOSIS_STAGES.length)).toBe(6);
    expect(nextDiagnosisQueueIndex(6, SITE_DIAGNOSIS_STAGES.length)).toBeNull();
  });
  it('uses status icons instead of visible stage numbers', () => {
    expect(diagnosisQueueMarker('pending')).toBe('·');
    expect(diagnosisQueueMarker('active')).toBe('…');
    expect(diagnosisQueueMarker('complete', 'success')).toBe('✓');
    expect(diagnosisQueueMarker('complete', 'failure')).toBe('×');
  });
});
