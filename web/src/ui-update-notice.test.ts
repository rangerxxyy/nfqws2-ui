import { describe, expect, it } from 'vitest';
import { uiUpdateNoticePresentation } from './ui-update-notice';

describe('background UI update notice', () => {
  it('highlights and notifies once for an available undismissed tag', () => {
    expect(uiUpdateNoticePresentation({ updateAvailable: true, latestVersion: 'v0.3.5' }, null)).toEqual({
      highlight: true, notify: true, version: 'v0.3.5',
    });
  });

  it('keeps the blue highlight but suppresses a dismissed tag', () => {
    expect(uiUpdateNoticePresentation({ updateAvailable: true, latestVersion: 'v0.3.5' }, 'v0.3.5')).toEqual({
      highlight: true, notify: false, version: 'v0.3.5',
    });
  });

  it('notifies again for a different release tag and clears when current', () => {
    expect(uiUpdateNoticePresentation({ updateAvailable: true, latestVersion: 'v0.3.6' }, 'v0.3.5').notify).toBe(true);
    expect(uiUpdateNoticePresentation({ updateAvailable: false, latestVersion: 'v0.3.5' }, 'v0.3.5')).toEqual({
      highlight: false, notify: false, version: null,
    });
  });
});
