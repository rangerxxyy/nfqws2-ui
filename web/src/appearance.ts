export type UiTheme = 'dark' | 'light' | 'neon';
export type UiLayout = 'standard' | 'compact' | 'dense' | 'wide';
export type AppearanceSettings = { theme: UiTheme; layout: UiLayout; remindWindowsTcpTimestamps: boolean };

const STORAGE_KEY = 'nfqws2-ui-appearance-v1';
const DEFAULTS: AppearanceSettings = { theme: 'dark', layout: 'standard', remindWindowsTcpTimestamps: true };

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
    layout: value.layout === 'compact' || value.layout === 'dense' || value.layout === 'wide' ? value.layout : 'standard',
    remindWindowsTcpTimestamps: value.remindWindowsTcpTimestamps !== false,
  };
}

export function saveAppearance(storage: Storage | null, settings: AppearanceSettings): AppearanceSettings {
  const normalized: AppearanceSettings = {
    theme: settings.theme === 'light' || settings.theme === 'neon' ? settings.theme : 'dark',
    layout: settings.layout === 'compact' || settings.layout === 'dense' || settings.layout === 'wide' ? settings.layout : 'standard',
    remindWindowsTcpTimestamps: settings.remindWindowsTcpTimestamps !== false,
  };
  try { storage?.setItem(STORAGE_KEY, JSON.stringify(normalized)); } catch { /* Appearance still applies for this session. */ }
  return normalized;
}

export function applyAppearance(settings: AppearanceSettings, root: HTMLElement = document.documentElement): void {
  root.dataset.theme = settings.theme;
  root.dataset.layout = settings.layout;
}

export function initializeAppearance(): AppearanceSettings {
  let storage: Storage | null = null;
  try { storage = window.localStorage; } catch { /* The default appearance is still usable when storage is blocked. */ }
  const settings = loadAppearance(storage);
  applyAppearance(settings);
  return settings;
}
