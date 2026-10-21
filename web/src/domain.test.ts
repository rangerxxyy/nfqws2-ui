import { describe, expect, it } from 'vitest';
import { actionToApplyConfig, configEditorReducer, formatBytes, isMutatingActionAllowed, normalizeStatus } from './domain';

describe('UI domain formatting', () => {
  it('keeps the configuration read-only until editing is requested and discards drafts on cancel', () => {
    const loaded = configEditorReducer({ draft: '', editing: false }, { type: 'loaded', content: 'original\n' });
    expect(loaded).toEqual({ draft: 'original\n', editing: false });

    const editing = configEditorReducer(loaded, { type: 'edit' });
    expect(editing.editing).toBe(true);
    const changed = configEditorReducer(editing, { type: 'change', value: 'modified\n' });
    expect(changed.draft).toBe('modified\n');
    expect(configEditorReducer(changed, { type: 'cancel', content: 'original\n' })).toEqual({
      draft: 'original\n', editing: false,
    });
  });

  it('keeps unknown status unknown instead of painting it green', () => {
    expect(normalizeStatus('unknown')).toBe('unknown');
    expect(normalizeStatus('running')).toBe('running');
    expect(normalizeStatus('unexpected')).toBe('unknown');
  });

  it('formats bytes without confusing total with available memory', () => {
    expect(formatBytes(512 * 1024 * 1024)).toBe('512 МБ');
    expect(formatBytes(null)).toBe('—');
  });

  it('allows only the service action enum', () => {
    expect(isMutatingActionAllowed('restart')).toBe(true);
    expect(isMutatingActionAllowed('reload; reboot')).toBe(false);
  });

  it('applies a saved config with the safe action for the confirmed service state', () => {
    expect(actionToApplyConfig('running')).toBe('restart');
    expect(actionToApplyConfig('stopped')).toBe('start');
    expect(actionToApplyConfig('unknown')).toBeNull();
    expect(actionToApplyConfig('error')).toBeNull();
  });
});
