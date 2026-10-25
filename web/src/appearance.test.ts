import { describe, expect, it } from 'vitest';
import { applyAppearance, loadAppearance, saveAppearance } from './appearance';

describe('appearance settings', () => {
  it('defaults the Windows TCP timestamp reminder on for old saved settings', () => {
    const storage = { getItem: () => JSON.stringify({ theme: 'dark', layout: 'standard' }) } as unknown as Storage;
    expect(loadAppearance(storage)).toEqual({ theme: 'dark', layout: 'standard', accent: 'blue', statusBar: 'standard', remindWindowsTcpTimestamps: true });
  });

  it('persists a user disabled timestamp reminder without changing theme or layout', () => {
    let value = '';
    const storage = { setItem: (_key: string, next: string) => { value = next; } } as unknown as Storage;
    saveAppearance(storage, { theme: 'neon', layout: 'compact', accent: 'cyan', statusBar: 'expanded', remindWindowsTcpTimestamps: false });
    expect(JSON.parse(value)).toEqual({ theme: 'neon', layout: 'compact', accent: 'cyan', statusBar: 'expanded', remindWindowsTcpTimestamps: false });
    expect(loadAppearance({ getItem: () => value } as unknown as Storage)).toEqual({ theme: 'neon', layout: 'compact', accent: 'cyan', statusBar: 'expanded', remindWindowsTcpTimestamps: false });
  });

  it('falls back to safe presets when stored accent and layout values are unknown', () => {
    const storage = { getItem: () => JSON.stringify({ theme: 'bogus', layout: 'bogus', accent: 'bogus', statusBar: 'bogus' }) } as unknown as Storage;
    expect(loadAppearance(storage)).toMatchObject({ theme: 'dark', layout: 'standard', accent: 'blue', statusBar: 'standard' });
  });

  it('applies the theme, accent, status bar and layout to the root element', () => {
    const root = { dataset: {} } as unknown as HTMLElement;
    applyAppearance({ theme: 'light', layout: 'tiles', accent: 'amber', statusBar: 'minimal', remindWindowsTcpTimestamps: true }, root);
    expect(root.dataset).toEqual({ theme: 'light', layout: 'tiles', accent: 'amber', statusBar: 'minimal' });
  });
});
