type Measurement = { classification: string };
type ProgressTarget = { label: string; address: string | null; direct: Measurement; current: Measurement; freezeDetected: boolean };
type ProgressStep = { label: string; state: 'complete' | 'active' | 'pending' };

export function freezeProgressPresentation(input: {
  stage: string;
  current: string | null;
  done: number;
  total: number;
  results: ProgressTarget[];
}): { headline: string; detail: string; percent: number; steps: ProgressStep[] } {
  const current = input.current ?? '';
  const total = Math.max(0, input.total);
  const percent = total === 0 ? 0 : Math.min(100, Math.round((input.done / total) * 100));

  if (input.stage === 'strategy_search') {
    const targetName = current.split(':', 1)[0] || 'сеть';
    const targets = input.results.length;
    const affected = Math.max(1, total - targets);
    const currentNumber = Math.min(affected, Math.max(1, input.done - targets + 1));
    return {
      headline: `Подбор стратегии · ${currentNumber} из ${affected} · ${targetName}`,
      detail: `Проверяю fake-SNI на проблемном адресе. Вариант засчитывается только после трёх последовательных успешных проверок.`,
      percent,
      steps: [
        { label: 'IP, ASN и базовые проверки', state: 'complete' },
        { label: 'Подбор имени fake-SNI', state: 'active' },
        { label: 'Подтверждение 3 из 3', state: 'pending' },
      ],
    };
  }

  const [targetName, phase = ''] = current.split(':', 2);
  const target = input.results.find((result) => result.label === targetName);
  const complete = (value: string) => value !== '' && value !== 'pending';
  const addressDone = Boolean(target?.address);
  const directDone = complete(target?.direct.classification ?? '');
  const routeDone = complete(target?.current.classification ?? '');
  const finishedTarget = !phase && routeDone;
  const itemNumber = finishedTarget
    ? Math.max(1, Math.min(total, input.done))
    : Math.max(1, Math.min(total, input.done + 1));
  const headline = finishedTarget
    ? `Пункт ${itemNumber} из ${total} завершён · ${targetName || 'проверка'}`
    : `Пункт ${itemNumber} из ${total} · ${targetName || 'подготовка'}`;
  const activeStep = !addressDone ? 0 : !directDone ? 1 : !routeDone ? 2 : -1;
  const steps: ProgressStep[] = [
    { label: 'Определение IP и ASN', state: addressDone ? 'complete' : activeStep === 0 ? 'active' : 'pending' },
    { label: 'Запрос напрямую', state: directDone ? 'complete' : activeStep === 1 ? 'active' : 'pending' },
    { label: 'Проверка через текущий маршрут', state: routeDone ? 'complete' : activeStep === 2 ? 'active' : 'pending' },
  ];
  const activeLabel = steps[activeStep]?.label ?? 'Базовые сетевые проверки завершены';
  const detail = phase.includes('прямой запрос завершён')
    ? 'Прямой ответ получен; сравниваю его с запросом через текущий маршрут.'
    : phase.includes('текущий маршрут')
      ? 'Сравниваю прямое соединение и маршрут nfqws2, фиксируя IP и причину тайм-аута.'
      : `${activeLabel}: результат каждой сети появится в таблице по мере проверки.`;
  return { headline, detail, percent, steps };
}
