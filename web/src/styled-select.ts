export type StyledOption = { value: string; label: string; disabled: boolean };

export function nextEnabledOption(options: readonly StyledOption[], currentIndex: number, direction: 1 | -1): number {
  for (let offset = 1; offset <= options.length; offset += 1) {
    const index = (currentIndex + direction * offset + options.length * 2) % options.length;
    if (!options[index]?.disabled) return index;
  }
  return -1;
}

export function findEnabledOptionByPrefix(options: readonly StyledOption[], prefix: string, currentIndex: number): number {
  const needle = prefix.trim().toLocaleLowerCase();
  if (!needle || !options.length) return -1;
  for (let offset = 1; offset <= options.length; offset += 1) {
    const index = (currentIndex + offset + options.length) % options.length;
    const option = options[index];
    if (option && !option.disabled && option.label.toLocaleLowerCase().startsWith(needle)) return index;
  }
  return -1;
}

export function styledSelectMenuSide(triggerTop: number, triggerBottom: number, menuHeight: number, viewportHeight: number, edge = 8): 'top' | 'bottom' {
  const roomAbove = Math.max(0, triggerTop - edge);
  const roomBelow = Math.max(0, viewportHeight - triggerBottom - edge);
  if (roomBelow >= menuHeight || roomAbove <= roomBelow) return 'bottom';
  return 'top';
}

export function styledSelectMenuMaxHeight(availableHeight: number, maximum = 34 * 16): number {
  return Math.max(0, Math.min(maximum, availableHeight));
}

export function styledSelectMenuWidth(triggerWidth: number, minimumWidth: number, viewportWidth: number, edge = 8): number {
  const preferred = Math.max(triggerWidth, Number.isFinite(minimumWidth) ? minimumWidth : 0);
  return Math.max(0, Math.min(preferred, viewportWidth - edge * 2));
}

type StyledSelectState = {
  select: HTMLSelectElement;
  wrapper: HTMLDivElement;
  trigger: HTMLButtonElement;
  menu: HTMLDivElement;
  activeIndex: number;
  open: boolean;
  search: string;
  searchTimer: number | null;
  menuClosingTimer: number | null;
  observer: MutationObserver;
};

const states = new WeakMap<HTMLSelectElement, StyledSelectState>();
const activeStates = new Set<StyledSelectState>();
let outsideHandlerInstalled = false;
let selectSequence = 0;

function positionMenu(state: StyledSelectState) {
  const rect = state.trigger.getBoundingClientRect();
  const width = styledSelectMenuWidth(rect.width, Number(state.select.dataset.menuMinWidth ?? 0), window.innerWidth);
  const left = Math.min(Math.max(8, rect.left), Math.max(8, window.innerWidth - width - 8));
  state.menu.style.maxHeight = '';
  const side = styledSelectMenuSide(rect.top, rect.bottom, state.menu.offsetHeight, window.innerHeight);
  state.menu.dataset.side = side;
  state.menu.style.left = `${left}px`;
  state.menu.style.width = `${width}px`;
  state.menu.style.top = side === 'bottom' ? `${rect.bottom + 4}px` : 'auto';
  state.menu.style.bottom = side === 'top' ? `${window.innerHeight - rect.top + 4}px` : 'auto';
  const available = side === 'top' ? rect.top - 12 : window.innerHeight - rect.bottom - 12;
  state.menu.style.maxHeight = `${styledSelectMenuMaxHeight(available)}px`;
}

function repositionOpenMenus() {
  for (const state of activeStates) if (state.open) positionMenu(state);
}

function optionModels(select: HTMLSelectElement): StyledOption[] {
  return [...select.options].map((option) => ({
    value: option.value,
    label: option.label || option.textContent?.trim() || option.value,
    disabled: option.disabled,
  }));
}

function selectedLabel(select: HTMLSelectElement): string {
  const selected = select.selectedOptions[0];
  return selected ? (selected.label || selected.textContent?.trim() || selected.value) : '';
}

function setActiveOption(state: StyledSelectState, index: number) {
  state.activeIndex = index;
  const option = state.menu.children.item(index);
  if (option instanceof HTMLElement) {
    state.trigger.setAttribute('aria-activedescendant', option.id);
    option.scrollIntoView({ block: 'nearest' });
  } else {
    state.trigger.removeAttribute('aria-activedescendant');
  }
}

function closeMenu(state: StyledSelectState) {
  state.open = false;
  if (state.menuClosingTimer !== null) window.clearTimeout(state.menuClosingTimer);
  state.menu.dataset.closing = 'true';
  state.menu.setAttribute('aria-hidden', 'true');
  state.menuClosingTimer = window.setTimeout(() => {
    state.menuClosingTimer = null;
    if (!state.open) state.menu.hidden = true;
    delete state.menu.dataset.closing;
  }, 150);
  state.trigger.setAttribute('aria-expanded', 'false');
  state.trigger.removeAttribute('aria-activedescendant');
  state.trigger.classList.remove('styled-select-trigger-open');
}

function chooseOption(state: StyledSelectState, index: number) {
  const option = state.select.options.item(index);
  if (!option || option.disabled || state.select.disabled) return;
  const changed = state.select.value !== option.value;
  state.select.value = option.value;
  closeMenu(state);
  state.trigger.focus();
  if (changed) state.select.dispatchEvent(new Event('change', { bubbles: true }));
}

function refreshState(state: StyledSelectState) {
  const { select, trigger, menu } = state;
  trigger.disabled = select.matches(':disabled');
  trigger.setAttribute('aria-expanded', String(state.open));
  trigger.textContent = selectedLabel(select) || 'Выберите вариант';
  trigger.classList.toggle('styled-select-trigger-open', state.open);
  menu.replaceChildren();
  const models = optionModels(select);
  models.forEach((option, index) => {
    const item = document.createElement('div');
    item.id = `${trigger.id}-option-${index}`;
    item.className = 'styled-select-option';
    item.setAttribute('role', 'option');
    item.setAttribute('aria-selected', String(index === select.selectedIndex));
    item.setAttribute('aria-disabled', String(option.disabled));
    item.dataset.active = String(state.open && index === state.activeIndex);
    item.dataset.selected = String(index === select.selectedIndex);
    item.textContent = option.label;
    if (index === select.selectedIndex) {
      const indicator = document.createElement('span');
      indicator.className = 'styled-select-indicator';
      indicator.setAttribute('aria-hidden', 'true');
      const check = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
      check.setAttribute('viewBox', '0 0 24 24');
      check.setAttribute('fill', 'none');
      check.setAttribute('stroke', 'currentColor');
      check.setAttribute('stroke-width', '3');
      check.setAttribute('stroke-linecap', 'round');
      check.setAttribute('stroke-linejoin', 'round');
      const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
      path.setAttribute('d', 'm5 12 4 4L19 6');
      check.append(path);
      indicator.append(check);
      item.append(indicator);
    }
    if (!option.disabled) item.addEventListener('click', () => chooseOption(state, index));
    menu.append(item);
  });
  if (state.open) {
    if (state.menuClosingTimer !== null) window.clearTimeout(state.menuClosingTimer);
    state.menuClosingTimer = null;
    delete menu.dataset.closing;
    menu.hidden = false;
    menu.removeAttribute('aria-hidden');
    const active = models[state.activeIndex];
    if (active && !active.disabled) {
      setActiveOption(state, state.activeIndex);
    } else {
      state.trigger.removeAttribute('aria-activedescendant');
    }
    positionMenu(state);
  }
}

function copySelectStyles(select: HTMLSelectElement, wrapper: HTMLDivElement, trigger: HTMLButtonElement) {
  const style = getComputedStyle(select);
  wrapper.style.position = 'relative';
  wrapper.style.display = 'block';
  wrapper.style.width = select.classList.contains('w-full') ? '100%' : style.width;
  wrapper.style.margin = `${style.marginTop} ${style.marginRight} ${style.marginBottom} ${style.marginLeft}`;
  wrapper.style.minWidth = style.minWidth;
  wrapper.style.flex = style.flex;
  trigger.style.width = '100%';
  trigger.style.height = style.height;
  trigger.style.minHeight = style.minHeight;
  trigger.style.boxSizing = 'border-box';
  trigger.style.padding = style.padding;
  trigger.style.color = style.color;
  trigger.style.backgroundColor = style.backgroundColor;
  trigger.style.backgroundImage = style.backgroundImage;
  trigger.style.border = style.border;
  trigger.style.borderRadius = style.borderRadius;
  trigger.style.fontFamily = style.fontFamily;
  trigger.style.fontSize = style.fontSize;
  trigger.style.fontWeight = style.fontWeight;
  trigger.style.lineHeight = style.lineHeight;
}

function installOne(select: HTMLSelectElement) {
  if (states.has(select)) return;
  const nativeValue = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value');
  const nativeDisabled = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'disabled');
  if (!nativeValue?.get || !nativeValue.set) return;

  const wrapper = document.createElement('div');
  wrapper.className = 'styled-select-shell';
  const trigger = document.createElement('button');
  trigger.type = 'button';
  trigger.className = 'styled-select-trigger';
  trigger.id = `${select.id || `styledSelect${++selectSequence}`}-trigger`;
  trigger.setAttribute('role', 'combobox');
  trigger.setAttribute('aria-haspopup', 'listbox');
  trigger.setAttribute('aria-expanded', 'false');
  const menu = document.createElement('div');
  menu.className = 'styled-select-menu';
  menu.id = `${trigger.id}-listbox`;
  menu.setAttribute('role', 'listbox');
  menu.hidden = true;
  menu.setAttribute('aria-hidden', 'true');
  trigger.setAttribute('aria-controls', menu.id);

  const accessibleLabel = select.getAttribute('aria-label');
  const labels = [...document.querySelectorAll('label')].filter((label) => label.htmlFor === select.id);
  if (labels[0]) {
    if (!labels[0].id) labels[0].id = `${trigger.id}-label`;
    trigger.setAttribute('aria-labelledby', labels.map((label) => {
      if (!label.id) label.id = `${trigger.id}-label-${labels.indexOf(label)}`;
      label.htmlFor = trigger.id;
      return label.id;
    }).join(' '));
  } else {
    trigger.setAttribute('aria-label', accessibleLabel || selectedLabel(select) || 'Выбор варианта');
  }

  const parent = select.parentElement;
  if (!parent) return;
  copySelectStyles(select, wrapper, trigger);
  parent.insertBefore(wrapper, select);
  wrapper.append(trigger, select);
  document.body.append(menu);
  select.classList.add('styled-select-native');
  select.tabIndex = -1;
  select.setAttribute('aria-hidden', 'true');
  select.style.position = 'absolute';
  select.style.width = '1px';
  select.style.height = '1px';
  select.style.opacity = '0';
  select.style.pointerEvents = 'none';
  select.style.clipPath = 'inset(50%)';
  select.style.margin = '0';
  const chevron = wrapper.nextElementSibling;
  if (chevron instanceof SVGElement && chevron.classList.contains('trace-protocol-chevron')) chevron.setAttribute('hidden', '');

  const state: StyledSelectState = { select, wrapper, trigger, menu, activeIndex: select.selectedIndex, open: false, search: '', searchTimer: null, menuClosingTimer: null, observer: new MutationObserver(() => refreshState(state)) };
  states.set(select, state);
  activeStates.add(state);
  state.observer.observe(select, { attributes: true, childList: true, subtree: true, characterData: true, attributeFilter: ['disabled', 'label', 'value', 'selected'] });
  Object.defineProperty(select, 'value', {
    configurable: true,
    enumerable: nativeValue.enumerable,
    get: () => nativeValue.get?.call(select) ?? '',
    set: (value: string) => { nativeValue.set?.call(select, value); refreshState(state); },
  });
  if (nativeDisabled?.get && nativeDisabled.set) {
    Object.defineProperty(select, 'disabled', {
      configurable: true,
      enumerable: nativeDisabled.enumerable,
      get: () => nativeDisabled.get?.call(select) ?? false,
      set: (value: boolean) => { nativeDisabled.set?.call(select, value); refreshState(state); },
    });
  }
  trigger.addEventListener('click', () => {
    if (state.open) closeMenu(state);
    else {
      state.open = true;
      state.activeIndex = -1;
      refreshState(state);
    }
  });
  trigger.addEventListener('keydown', (event) => {
    const models = optionModels(select);
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const direction = event.key === 'ArrowDown' ? 1 : -1;
      if (!state.open) state.open = true;
      setActiveOption(state, nextEnabledOption(models, state.activeIndex < 0 ? select.selectedIndex : state.activeIndex, direction));
      refreshState(state);
    } else if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault();
      if (!state.open) state.open = true;
      const indexes = models.map((option, index) => option.disabled ? -1 : index).filter((index) => index >= 0);
      setActiveOption(state, event.key === 'Home' ? (indexes[0] ?? -1) : (indexes.at(-1) ?? -1));
      refreshState(state);
    } else if (event.key === 'Escape') {
      if (state.open) { event.preventDefault(); closeMenu(state); }
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (!state.open) {
        state.open = true;
        state.activeIndex = select.selectedIndex;
        refreshState(state);
      } else chooseOption(state, state.activeIndex);
    } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      state.search += event.key;
      if (state.searchTimer !== null) window.clearTimeout(state.searchTimer);
      state.searchTimer = window.setTimeout(() => { state.search = ''; state.searchTimer = null; }, 750);
      const match = findEnabledOptionByPrefix(models, state.search, select.selectedIndex);
      if (match >= 0) {
        if (!state.open) state.open = true;
        setActiveOption(state, match);
        refreshState(state);
      }
    }
  });
  if (!outsideHandlerInstalled) {
    document.addEventListener('pointerdown', (event) => {
      const target = event.target;
      for (const current of activeStates) {
        if (current.open && target instanceof Node && !current.wrapper.contains(target) && !current.menu.contains(target)) closeMenu(current);
      }
    });
    document.addEventListener('scroll', repositionOpenMenus, true);
    window.addEventListener('resize', repositionOpenMenus);
    outsideHandlerInstalled = true;
  }
  refreshState(state);
}

export function refreshStyledSelect(select: HTMLSelectElement): void {
  const state = states.get(select);
  if (state) refreshState(state);
}

export function installStyledSelects(root: ParentNode = document): void {
  // Dynamic editors replace their controls on each render. Drop detached
  // menus and observers so repeated profile edits do not retain stale state.
  for (const state of activeStates) {
    if (state.select.isConnected) continue;
    state.observer.disconnect();
    if (state.searchTimer !== null) window.clearTimeout(state.searchTimer);
    if (state.menuClosingTimer !== null) window.clearTimeout(state.menuClosingTimer);
    state.menu.remove();
    activeStates.delete(state);
  }
  root.querySelectorAll<HTMLSelectElement>('select').forEach(installOne);
}
