import { describe, expect, it } from 'vitest';
import { snapshotFreeSpaceState } from './snapshot-storage';

describe('snapshotFreeSpaceState', () => {
  it('warns when the filesystem has less room than the largest accepted archive', () => {
    expect(snapshotFreeSpaceState(64 * 1024 * 1024 - 1, 64 * 1024 * 1024)).toBe('low');
  });

  it('accepts exactly enough room for the largest accepted archive', () => {
    expect(snapshotFreeSpaceState(64 * 1024 * 1024, 64 * 1024 * 1024)).toBe('sufficient');
  });

  it('keeps unavailable filesystem measurements distinct from zero free space', () => {
    expect(snapshotFreeSpaceState(null, 64 * 1024 * 1024)).toBe('unknown');
    expect(snapshotFreeSpaceState(0, 64 * 1024 * 1024)).toBe('low');
  });
});
