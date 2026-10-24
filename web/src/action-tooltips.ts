const TOOLTIP_CLASS = 'action-tooltip';
export const DEFAULT_ACTION_TOOLTIP_DELAY_MS = 3000;
export function scheduleDelayedTooltip(callback: () => void, delayMs = DEFAULT_ACTION_TOOLTIP_DELAY_MS): () => void {
  const timer = globalThis.setTimeout(callback, delayMs);
  return () => globalThis.clearTimeout(timer);
}

export function installActionTooltips(root: Document = document, delayMs = DEFAULT_ACTION_TOOLTIP_DELAY_MS): void {
  let cancelDelay: (() => void) | null = null;
  let active: HTMLElement | null = null;
  let tooltip: HTMLDivElement | null = null;
  const transfer = (element: Element) => {
    if (!(element instanceof HTMLButtonElement)) return;
    const title = element.getAttribute('title');
    if (title?.trim()) {
      element.dataset.actionTooltip = title.trim();
      element.removeAttribute('title');
    }
  };
  const findButton = (target: EventTarget | null) => target instanceof Element ? target.closest<HTMLButtonElement>('button[data-action-tooltip], button[data-action-click][data-action-tooltip]') : null;
  const clear = () => {
    cancelDelay?.(); cancelDelay = null;
    active?.removeAttribute('aria-describedby');
    active = null;
    tooltip?.remove(); tooltip = null;
  };
  const schedule = (button: HTMLButtonElement | null) => {
    clear();
    if (!button || button.disabled || !button.dataset.actionTooltip) return;
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
      tooltip.style.top = `${Math.max(8, rect.bottom + 8)}px`;
      active.setAttribute('aria-describedby', tooltip.id);
      cancelDelay = null;
    }, delayMs);
  };
  root.querySelectorAll('button[title]').forEach(transfer);
  const observer = new MutationObserver((records) => {
    for (const record of records) {
      if (record.type === 'attributes' && record.target instanceof Element) transfer(record.target);
      if (record.type === 'childList') record.addedNodes.forEach((node) => {
        if (node instanceof Element) { transfer(node); node.querySelectorAll('button[title]').forEach(transfer); }
      });
    }
  });
  observer.observe(root.documentElement, { subtree: true, childList: true, attributes: true, attributeFilter: ['title'] });
  root.addEventListener('pointerover', (event) => {
    const button = findButton(event.target);
    if (button && (!button.contains(event.relatedTarget as Node | null))) schedule(button);
  });
  root.addEventListener('pointerout', (event) => {
    const button = findButton(event.target);
    if (button && !button.contains(event.relatedTarget as Node | null) && button === active) clear();
  });
  root.addEventListener('focusin', (event) => schedule(findButton(event.target)));
  root.addEventListener('focusout', (event) => {
    const button = findButton(event.target);
    if (button && button === active) clear();
  });
}
