import { describe, expect, it } from 'vitest';
import { freezeProgressPresentation } from './freeze-progress';

describe('16 KB diagnostic compact progress', () => {
  it('summarizes the live network and route stage in one status line', () => {
    const view = freezeProgressPresentation({
      stage: 'baseline', current: 'OVH: проверка через текущий маршрут', done: 1, total: 20,
    });
    expect(view.headline).toBe('Проверяю сети без обхода');
    expect(view.count).toBe('1 из 20');
    expect(view.detail).toBe('Сейчас: OVH: проверка через текущий маршрут');
  });

  it('shows the matching-name phase and total progress count', () => {
    const view = freezeProgressPresentation({
      stage: 'strategy_search', current: 'OVH: проверка SNI ya.ru (1)', done: 20, total: 24,
    });
    expect(view.headline).toBe('Подбираю имена для сетей с обрывом');
    expect(view.count).toBe('20 из 24');
    expect(view.detail).toBe('Сейчас: OVH: проверка SNI ya.ru (1)');
  });
});
