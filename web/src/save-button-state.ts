export type SaveButtonState = { dirtyState: 'clean' | 'changed'; disabled: boolean };

export function draftDiffersFromBaseline(current: unknown, baseline: unknown): boolean {
  if (Object.is(current, baseline)) return false;
  if (Array.isArray(current) || Array.isArray(baseline)) {
    return !Array.isArray(current) || !Array.isArray(baseline) || current.length !== baseline.length || current.some((value, index) => draftDiffersFromBaseline(value, baseline[index]));
  }
  if (current && baseline && typeof current === 'object' && typeof baseline === 'object') {
    const currentKeys = Object.keys(current).sort();
    const baselineKeys = Object.keys(baseline).sort();
    return currentKeys.length !== baselineKeys.length
      || currentKeys.some((key, index) => key !== baselineKeys[index] || draftDiffersFromBaseline((current as Record<string, unknown>)[key], (baseline as Record<string, unknown>)[key]));
  }
  return true;
}

export function saveButtonState(input: { loaded: boolean; writable: boolean; dirty: boolean; busy: boolean }): SaveButtonState {
  return {
    dirtyState: input.dirty ? 'changed' : 'clean',
    disabled: input.busy || !input.loaded || !input.writable || !input.dirty,
  };
}
