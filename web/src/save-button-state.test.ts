import { describe, expect, it } from 'vitest';
import { draftDiffersFromBaseline, saveButtonState } from './save-button-state';

describe('saveButtonState', () => {
  it('disables and marks a loaded unchanged form as clean', () => {
    expect(saveButtonState({ loaded: true, writable: true, dirty: false, busy: false })).toEqual({
      dirtyState: 'clean',
      disabled: true,
    });
  });

  it('enables a writable dirty form', () => {
    expect(saveButtonState({ loaded: true, writable: true, dirty: true, busy: false })).toEqual({
      dirtyState: 'changed',
      disabled: false,
    });
  });

  it('keeps a dirty form disabled while saving or when it is not writable', () => {
    expect(saveButtonState({ loaded: true, writable: true, dirty: true, busy: true }).disabled).toBe(true);
    expect(saveButtonState({ loaded: true, writable: false, dirty: true, busy: false }).disabled).toBe(true);
  });

  it('disables controls until their saved values have loaded', () => {
    expect(saveButtonState({ loaded: false, writable: true, dirty: false, busy: false })).toEqual({
      dirtyState: 'clean',
      disabled: true,
    });
  });
});

describe('draftDiffersFromBaseline', () => {
  it('treats edits reverted to the baseline as clean', () => {
    const original = [['--filter-tcp=443', '--lua-desync=fake']];
    const edited = [['--filter-tcp=443', '--lua-desync=multisplit']];
    expect(draftDiffersFromBaseline(edited, original)).toBe(true);
    expect(draftDiffersFromBaseline(original, original)).toBe(false);
  });

  it('compares nested settings independent of object key order', () => {
    expect(draftDiffersFromBaseline({ first: ['a'], second: 2 }, { second: 2, first: ['a'] })).toBe(false);
    expect(draftDiffersFromBaseline({ first: ['b'] }, { first: ['a'] })).toBe(true);
  });
});
