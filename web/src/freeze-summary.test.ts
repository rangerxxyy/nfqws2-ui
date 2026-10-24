import { describe, expect, it } from 'vitest';
import { summarizeFreezeScan } from './freeze-summary';

const result = (freezeDetected: boolean, found: boolean) => ({
  freezeDetected,
  foundStrategy: found ? {} : null,
}) as never;

describe('summarizeFreezeScan', () => {
  it('summarizes affected networks and confirmed names after a complete run', () => {
    expect(summarizeFreezeScan({
      state: 'complete',
      message: 'Проверка завершена',
      results: [result(true, true), result(true, false), result(false, false)],
    })).toEqual({ tone: 'warning', text: 'Обрыв есть в 2 сетях из 3. Для 1 сети имя прошло три проверки; найденный вариант показан в таблице.' });
  });

  it('does not describe a partial scan as a completed diagnosis', () => {
    expect(summarizeFreezeScan({
      state: 'partial',
      message: 'Достигнут лимит времени',
      results: [result(true, true), result(false, false)],
    })).toEqual({ tone: 'neutral', text: 'Частичная проверка · возможный обрыв в 1 из 2 сетей · Достигнут лимит времени' });
  });

  it('reports a completed run with no observed freeze', () => {
    expect(summarizeFreezeScan({ state: 'complete', message: 'Готово', results: [result(false, false)] }))
      .toEqual({ tone: 'success', text: 'Повторяемый обрыв не обнаружен в 1 проверенной сети.' });
  });
});
