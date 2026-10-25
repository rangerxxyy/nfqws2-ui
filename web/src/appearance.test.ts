import { describe, expect, it } from 'vitest';
import { loadAppearance, saveAppearance } from './appearance';

describe('appearance settings', () => {
  it('defaults the Windows TCP timestamp reminder on for old saved settings', () => {
    const storage = { getItem: () => JSON.stringify({ theme: 'dark', layout: 'standard' }) } as unknown as Storage;
    expect(loadAppearance(storage).remindWindowsTcpTimestamps).toBe(true);
  });

  it('persists a user disabled timestamp reminder without changing theme or layout', () => {
    let value = '';
    const storage = { setItem: (_key: string, next: string) => { value = next; } } as unknown as Storage;
    saveAppearance(storage, { theme: 'neon', layout: 'compact', remindWindowsTcpTimestamps: false });
    expect(JSON.parse(value)).toEqual({ theme: 'neon', layout: 'compact', remindWindowsTcpTimestamps: false });
    expect(loadAppearance({ getItem: () => value } as unknown as Storage)).toEqual({ theme: 'neon', layout: 'compact', remindWindowsTcpTimestamps: false });
  });
});
