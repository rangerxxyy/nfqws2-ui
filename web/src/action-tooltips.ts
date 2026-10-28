const TOOLTIP_CLASS = 'action-tooltip';
export const DEFAULT_ACTION_TOOLTIP_DELAY_MS = 2000;
export function actionTooltipDelayMs(button: Pick<HTMLButtonElement, 'dataset'>, fallback = DEFAULT_ACTION_TOOLTIP_DELAY_MS): number {
  const delay = Number(button.dataset.actionTooltipDelayMs);
  return Number.isInteger(delay) && delay >= 0 && delay <= 10000 ? delay : fallback;
}
export function scheduleDelayedTooltip(callback: () => void, delayMs = DEFAULT_ACTION_TOOLTIP_DELAY_MS): () => void {
  const timer = globalThis.setTimeout(callback, delayMs);
  return () => globalThis.clearTimeout(timer);
}

export function shouldShowTooltipForFocus(button: Pick<Element, 'matches'> | null): boolean {
  return Boolean(button?.matches(':focus-visible'));
}

const ACTION_DESCRIPTIONS: Record<string, string> = {
  addMonitoringSite: 'Добавить домен в список ручного мониторинга.',
  runMonitoring: 'Проверить доступность сохранённых доменов.',
  saveActiveListFile: 'Сохранить изменения в выбранном файле списка.',
  applyConfigAndRestart: 'Проверить конфигурацию, сохранить её и перезапустить nfqws2.',
  openUpdateModal: 'Показать версии выбранного компонента и установить обновление.',
  toggleDaemonPower: 'Запустить или остановить службу nfqws2.',
  restartService: 'Перезапустить службу nfqws2.',
  runSiteDiagnosis: 'Сравнить прямое подключение к сайту с текущим маршрутом.',
  runFreezeScan: 'Проверить сайты и сети на возможные обрывы передачи.',
  startStrategyPick: 'Подобрать и проверить стратегии для выбранного домена.',
  copyConfigText: 'Скопировать полный текст конфигурации.',
  openListImport: 'Загрузить файл и добавить его строки в черновик списка.',
  exportListDraft: 'Скачать текущий список в файл.',
  toggleConfigOutline: 'Показать или скрыть навигацию по строкам конфигурации.',
  requestDeleteCustomList: 'Открыть подтверждение удаления пользовательского списка.',
  confirmDeleteCustomList: 'Удалить выбранный пользовательский файл списка.',
  dismissUiUpdateNotice: 'Скрыть напоминание об этой версии.',
  closeUpdateModal: 'Закрыть список доступных версий.',
};
const ROUTE_DESCRIPTIONS: Record<string, string> = {
  conf: 'Открыть конфигурацию и параметры nfqws2.',
  lists: 'Открыть стандартные и пользовательские списки доменов.',
  check: 'Открыть диагностику доступности сайтов.',
  trace: 'Открыть трассировку соединения к домену.',
  autotune: 'Подобрать рабочую стратегию для сайта.',
  monitor: 'Открыть ручную проверку сохранённых доменов.',
};

export function fallbackTooltipText(button: HTMLButtonElement): string {
  const explicit = button.dataset.actionTooltip?.trim() || button.getAttribute('aria-label')?.trim();
  if (explicit) return explicit;
  const action = button.dataset.actionClick || button.dataset.action;
  if (action === 'switchMainTab') {
    try {
      const [route] = JSON.parse(button.dataset.args || '[]') as [string?];
      if (route && ROUTE_DESCRIPTIONS[route]) return ROUTE_DESCRIPTIONS[route];
    } catch { /* Fall back to visible text. */ }
  }
  if (action && ACTION_DESCRIPTIONS[action]) return ACTION_DESCRIPTIONS[action];
  return button.textContent?.replace(/\s+/g, ' ').trim() || '';
}

export function installActionTooltips(root: Document = document, delayMs = DEFAULT_ACTION_TOOLTIP_DELAY_MS): void {
  let cancelDelay: (() => void) | null = null;
  let active: HTMLButtonElement | null = null;
  let tooltip: HTMLDivElement | null = null;
  const prepare = (element: Element) => {
    if (!(element instanceof HTMLButtonElement)) return;
    const title = element.getAttribute('title');
    if (title?.trim()) {
      element.dataset.actionTooltip = title.trim();
      element.removeAttribute('title');
    }
    if (!element.dataset.actionTooltip) element.dataset.actionTooltip = fallbackTooltipText(element);
  };
  const findButton = (target: EventTarget | null) => target instanceof Element
    ? target.closest<HTMLButtonElement>('button, .side-nav-btn')
    : null;
  const clear = () => {
    cancelDelay?.(); cancelDelay = null;
    active?.removeAttribute('aria-describedby');
    active = null;
    tooltip?.remove(); tooltip = null;
  };
  const clearActiveTooltip = (event: Event) => {
    if (findButton(event.target) === active) clear();
  };
  const schedule = (button: HTMLButtonElement | null) => {
    clear();
    if (!button || button.disabled) return;
    prepare(button);
    if (!button.dataset.actionTooltip) return;
    active = button;
    cancelDelay = scheduleDelayedTooltip(() => {
      if (!active || !document.contains(active)) return;
      tooltip = document.createElement('div');
      tooltip.className = TOOLTIP_CLASS;
      tooltip.id = `action-tooltip-${Math.random().toString(36).slice(2)}`;
      tooltip.setAttribute('role', 'tooltip');
      tooltip.textContent = active.dataset.actionTooltip || '';
      document.body.append(tooltip);
      const rect = active.getBoundingClientRect();
      tooltip.style.left = `${Math.min(Math.max(8, rect.left), Math.max(8, window.innerWidth - tooltip.offsetWidth - 8))}px`;
      const below = rect.bottom + 8;
      const top = below + tooltip.offsetHeight <= window.innerHeight - 8 ? below : Math.max(8, rect.top - tooltip.offsetHeight - 8);
      tooltip.style.top = `${top}px`;
      active.setAttribute('aria-describedby', tooltip.id);
      cancelDelay = null;
    }, actionTooltipDelayMs(button, delayMs));
  };
  root.querySelectorAll('button, .side-nav-btn').forEach(prepare);
  const observer = new MutationObserver((records) => {
    for (const record of records) {
      if (record.type === 'attributes' && record.target instanceof Element) prepare(record.target);
      if (record.type === 'childList') {
        record.removedNodes.forEach((node) => {
          if (active && node instanceof Node && node.contains(active)) clear();
        });
        record.addedNodes.forEach((node) => {
          if (node instanceof Element) {
            prepare(node);
            node.querySelectorAll('button, .side-nav-btn').forEach(prepare);
          }
        });
      }
    }
  });
  root.addEventListener('pointerover', (event) => {
    if ((event as PointerEvent).pointerType === 'touch' || window.matchMedia('(hover: none)').matches) return;
    const button = findButton(event.target);
    if (button && !button.contains(event.relatedTarget as Node | null)) schedule(button);
  });
  root.addEventListener('pointerout', (event) => {
    const button = findButton(event.target);
    if (button && !button.contains(event.relatedTarget as Node | null) && button === active) clear();
  });
  root.addEventListener('focusin', (event) => {
    const button = findButton(event.target);
    if (shouldShowTooltipForFocus(button)) schedule(button);
  });
  root.addEventListener('focusout', (event) => {
    const button = findButton(event.target);
    if (button && button === active) clear();
  });
  root.addEventListener('click', clearActiveTooltip, true);
  root.addEventListener('scroll', clear, true);
  root.defaultView?.addEventListener('resize', clear);
  observer.observe(root.documentElement, { subtree: true, childList: true, attributes: true, attributeFilter: ['title', 'aria-label'] });
}
