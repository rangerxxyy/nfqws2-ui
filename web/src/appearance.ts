export type UiTheme = 'dark' | 'light' | 'neon';
export type UiLayout = 'standard' | 'compact' | 'dense' | 'wide' | 'right' | 'top' | 'tiles' | 'simplified';
export type UiAccent = 'blue' | 'cyan' | 'emerald' | 'amber' | 'rose' | 'violet';
export type UiStatusBar = 'standard' | 'minimal' | 'compact' | 'stacked' | 'reverse';
export type AppearanceSettings = { theme: UiTheme; layout: UiLayout; accent: UiAccent; statusBar: UiStatusBar; remindWindowsTcpTimestamps: boolean; showConfigWarnings: boolean; wrapConfigLines?: boolean };

const STORAGE_KEY = 'nfqws2-ui-appearance-v1';
const DEFAULTS: AppearanceSettings = { theme: 'dark', layout: 'standard', accent: 'blue', statusBar: 'standard', remindWindowsTcpTimestamps: true, showConfigWarnings: true, wrapConfigLines: true };

function readStorage(storage: Storage | null): unknown {
  try {
    const value = storage?.getItem(STORAGE_KEY);
    return value ? JSON.parse(value) as unknown : null;
  } catch {
    return null;
  }
}

export function loadAppearance(storage: Storage | null): AppearanceSettings {
  const saved = readStorage(storage);
  if (!saved || typeof saved !== 'object') return { ...DEFAULTS };
  const value = saved as Partial<AppearanceSettings>;
  return {
    theme: value.theme === 'light' || value.theme === 'neon' ? value.theme : 'dark',
    layout: ['compact', 'dense', 'wide', 'right', 'top', 'tiles', 'simplified'].includes(value.layout ?? '') ? value.layout as UiLayout : 'standard',
    accent: ['cyan', 'emerald', 'amber', 'rose', 'violet'].includes(value.accent ?? '') ? value.accent as UiAccent : 'blue',
    statusBar: ['minimal', 'compact', 'stacked', 'reverse'].includes(value.statusBar ?? '') ? value.statusBar as UiStatusBar : 'standard',
    remindWindowsTcpTimestamps: value.remindWindowsTcpTimestamps !== false,
    showConfigWarnings: value.showConfigWarnings !== false,
    wrapConfigLines: value.wrapConfigLines !== false,
  };
}

export function saveAppearance(storage: Storage | null, settings: AppearanceSettings): AppearanceSettings {
  const normalized: AppearanceSettings = {
    theme: settings.theme === 'light' || settings.theme === 'neon' ? settings.theme : 'dark',
    layout: ['compact', 'dense', 'wide', 'right', 'top', 'tiles', 'simplified'].includes(settings.layout) ? settings.layout : 'standard',
    accent: ['cyan', 'emerald', 'amber', 'rose', 'violet'].includes(settings.accent) ? settings.accent : 'blue',
    statusBar: ['minimal', 'compact', 'stacked', 'reverse'].includes(settings.statusBar) ? settings.statusBar : 'standard',
    remindWindowsTcpTimestamps: settings.remindWindowsTcpTimestamps !== false,
    showConfigWarnings: settings.showConfigWarnings !== false,
    wrapConfigLines: settings.wrapConfigLines !== false,
  };
  try { storage?.setItem(STORAGE_KEY, JSON.stringify(normalized)); } catch { /* Appearance still applies for this session. */ }
  return normalized;
}

export function applyAppearance(settings: AppearanceSettings, root: HTMLElement = document.documentElement): void {
  root.dataset.theme = settings.theme;
  root.dataset.layout = settings.layout;
  root.dataset.accent = settings.accent;
  root.dataset.statusBar = settings.statusBar;
}

export function initializeAppearance(): AppearanceSettings {
  let storage: Storage | null = null;
  try { storage = window.localStorage; } catch { /* The default appearance is still usable when storage is blocked. */ }
  const settings = loadAppearance(storage);
  applyAppearance(settings);
  return settings;
}
