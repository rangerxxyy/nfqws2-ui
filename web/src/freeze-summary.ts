import type { FreezeScanOperation } from './api';

type FreezeScanSummaryInput = Pick<FreezeScanOperation, 'state' | 'message' | 'results'>;
type FreezeScanSummary = { tone: 'warning' | 'success' | 'neutral' | 'error'; text: string };

function russianCount(value: number, one: string, few: string, many: string) {
  const mod100 = value % 100;
  const mod10 = value % 10;
  if (mod100 >= 11 && mod100 <= 14) return `${value} ${many}`;
  if (mod10 === 1) return `${value} ${one}`;
  if (mod10 >= 2 && mod10 <= 4) return `${value} ${few}`;
  return `${value} ${many}`;
}

export function summarizeFreezeScan(operation: FreezeScanSummaryInput): FreezeScanSummary | null {
  if (!['complete', 'partial', 'cancelled', 'failed'].includes(operation.state)) return null;
  const scanned = operation.results.length;
  if (operation.state === 'failed') return { tone: 'error', text: operation.message };
  if (operation.state === 'cancelled') {
    return { tone: 'neutral', text: `Проба остановлена. Сохранены результаты ${russianCount(scanned, 'сети', 'сетей', 'сетей')}.` };
  }
  const affected = operation.results.filter((result) => result.freezeDetected).length;
  if (operation.state === 'partial') {
    return {
      tone: 'neutral',
      text: `Частичная проверка · возможный обрыв в ${affected} из ${scanned} сетей · ${operation.message}`,
    };
  }
  if (affected === 0) {
    return {
      tone: 'success',
      text: `Повторяемый обрыв не обнаружен в ${russianCount(scanned, 'проверенной сети', 'проверенных сетях', 'проверенных сетях')}.`,
    };
  }
  const found = operation.results.filter((result) => result.freezeDetected && result.foundStrategy).length;
  const names = found > 0
    ? ` Для ${russianCount(found, 'сети', 'сетей', 'сетей')} имя прошло три проверки; найденный вариант показан в таблице.`
    : ' Подтверждённый вариант не найден.';
  return {
    tone: 'warning',
    text: `Возможный обрыв в ${affected} ${russianCount(affected, 'сети', 'сетях', 'сетях').replace(/^\d+\s/, '')} из ${scanned}.${names}`,
  };
}
