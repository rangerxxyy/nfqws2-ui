export type CopyProfileDraftResult =
  | { ok: true; profiles: string[][] }
  | { ok: false; reason: 'profile-limit' | 'argument-limit' };

export function copyProfileIntoDraft(
  draft: readonly (readonly string[])[],
  sourceArgs: readonly string[],
): CopyProfileDraftResult {
  if (draft.length >= 64) return { ok: false, reason: 'profile-limit' };

  const copiedArgs = sourceArgs.filter((argument) => !argument.startsWith('--name='));
  const argumentCount = draft.reduce((total, profile) => total + profile.length, copiedArgs.length);
  if (argumentCount > 8192) return { ok: false, reason: 'argument-limit' };

  return {
    ok: true,
    profiles: [...draft.map((profile) => [...profile]), copiedArgs],
  };
}
