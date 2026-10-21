import { describe, expect, it } from 'vitest';
import { t, type MessageKey } from './i18n';

describe('interface translations', () => {
  it('provides the core operational copy in each supported language', () => {
    const keys: MessageKey[] = [
      'overview', 'config', 'logs', 'backups', 'settings', 'running', 'stopped', 'unknown',
      'start', 'stop', 'restart', 'refresh', 'save', 'confirmStop', 'confirmRestart',
      'configRestartHint', 'editConfig', 'cancelEdit', 'cancelEditConfirm', 'viewConfig', 'lightTheme', 'darkTheme',
    ];
    for (const locale of ['ru', 'en', 'zh', 'tr'] as const) {
      for (const key of keys) expect(t(locale, key).trim(), `${locale}.${key}`).not.toBe('');
    }
  });
});
