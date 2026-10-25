import { describe, expect, it } from 'vitest';
import { diagnosisQueueState, SITE_DIAGNOSIS_STAGES } from './diagnosis-stages';

describe('site diagnosis stage queue', () => {
  it('lists checks in their intended order before a request starts', () => {
    expect(SITE_DIAGNOSIS_STAGES.map(({ id }) => id)).toEqual(['dns', 'tcp', 'direct', 'sni', 'http', 'current', 'quic']);
  });
  it('marks earlier steps complete, one active, and later steps pending', () => {
    expect(SITE_DIAGNOSIS_STAGES.map((_, index) => diagnosisQueueState(index, 2))).toEqual(['complete', 'complete', 'active', 'pending', 'pending', 'pending', 'pending']);
  });
});
