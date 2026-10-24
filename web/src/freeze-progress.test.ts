import { describe, expect, it } from 'vitest';
import { freezeProgressPresentation } from './freeze-progress';

describe('16 KB diagnostic step progress', () => {
  it('shows the numbered network and highlights the current route check', () => {
    const view = freezeProgressPresentation({
      stage: 'baseline', current: 'OVH: проверка через текущий маршрут', done: 1, total: 20,
      results: [{ label: 'OVH', address: '141.95.207.211', direct: { classification: 'Complete' }, current: { classification: 'pending' }, freezeDetected: false }],
    });
    expect(view.headline).toBe('Пункт 2 из 20 · OVH');
    expect(view.steps.map((step) => step.state)).toEqual(['complete', 'complete', 'active']);
    expect(view.detail).toContain('Сравниваю прямое соединение и маршрут nfqws2');
  });

  it('explains the fake-SNI stage and its three-success confirmation rule', () => {
    const results = Array.from({ length: 20 }, (_, index) => ({
      label: index === 0 ? 'OVH' : `target-${index}`,
      address: '141.95.207.211',
      direct: { classification: 'TimeoutBeforeBody' },
      current: { classification: 'TimeoutBeforeBody' },
      freezeDetected: index < 2,
    }));
    const view = freezeProgressPresentation({
      stage: 'strategy_search', current: 'OVH: проверка SNI ya.ru (1)', done: 20, total: 22,
      results,
    });
    expect(view.headline).toBe('Подбор стратегии · 1 из 2 · OVH');
    expect(view.steps.map((step) => step.state)).toEqual(['complete', 'active', 'pending']);
    expect(view.detail).toContain('после трёх последовательных успешных проверок');
  });
});
