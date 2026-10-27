import { describe, expect, it } from 'vitest';
import { podkopWarningPresentation } from './podkop-warning';

describe('Podkop route warning', () => {
  it('explains the effect for one FakeIP-routed site', () => {
    expect(podkopWarningPresentation([' example.org '])).toEqual({
      title: 'example.org направляется через Podkop',
      detail: expect.stringContaining('не подходит для надёжного сравнения стратегий'),
    });
  });

  it('lists distinct affected sites and stays hidden when no FakeIP was detected', () => {
    expect(podkopWarningPresentation(['one.example', 'two.example', 'one.example'])?.title)
      .toBe('Через Podkop направляются: one.example, two.example');
    expect(podkopWarningPresentation([])).toBeNull();
    expect(podkopWarningPresentation(undefined)).toBeNull();
  });
});
