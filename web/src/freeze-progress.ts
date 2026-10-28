export function freezeProgressPresentation(input: {
  stage: string;
  current: string | null;
  done: number;
  total: number;
}): { headline: string; count: string; detail: string; percent: number } {
  const current = input.current ?? '';
  const total = Math.max(0, input.total);
  const done = Math.max(0, Math.min(total, input.done));
  const percent = total === 0 ? 0 : Math.round((done / total) * 100);
  const strategySearch = input.stage === 'strategy_search';
  return {
    headline: strategySearch ? 'Подбираю имена для сетей с обрывом' : 'Проверяю сети без обхода',
    count: total > 0 ? `${done} из ${total}` : 'подготовка…',
    detail: current ? `Сейчас: ${current}` : strategySearch
      ? 'Ищу имя fake-SNI, которое проходит проверку для этой сети.'
      : 'Подготавливаю проверочные соединения.',
    percent,
  };
}
