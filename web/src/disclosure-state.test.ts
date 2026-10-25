import { describe, expect, it } from 'vitest';
import { captureForeignConfigDisclosureState, formatForeignConfigProbeSummary } from './disclosure-state';

describe('foreign config disclosure state', () => {
  it('keeps each block independent when results are rebuilt', () => {
    const opened = captureForeignConfigDisclosureState([
      { open: true, dataset: { disclosureKey: 'current' } },
      { open: false, dataset: { disclosureKey: 'candidate-0' } },
      { open: true, dataset: { disclosureKey: 'candidate-1' } },
    ]);

    expect(opened).toEqual(new Set(['current', 'candidate-1']));
  });

  it('shows successful repeats and average time when available', () => {
    expect(formatForeignConfigProbeSummary(2, 3, 145)).toBe('2/3 успешных · 145 мс');
    expect(formatForeignConfigProbeSummary(0, 1, null)).toBe('0/1 успешных');
  });
});
