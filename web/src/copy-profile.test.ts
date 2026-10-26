import { describe, expect, it } from 'vitest';
import { copyProfileIntoDraft } from './copy-profile';

describe('copyProfileIntoDraft', () => {
  it('copies arguments in order without carrying over the source profile name', () => {
    const draft = [['--filter-tcp=443']];
    const source = ['--name=Managed profile', '--payload=tls_client_hello', '--lua-desync=fake'];

    const result = copyProfileIntoDraft(draft, source);

    expect(result).toEqual({
      ok: true,
      profiles: [
        ['--filter-tcp=443'],
        ['--payload=tls_client_hello', '--lua-desync=fake'],
      ],
    });
    expect(draft).toEqual([['--filter-tcp=443']]);
    expect(source[0]).toBe('--name=Managed profile');
  });

  it('refuses to create more than 64 custom profiles', () => {
    const draft = Array.from({ length: 64 }, () => ['--payload=tls_client_hello']);

    expect(copyProfileIntoDraft(draft, ['--lua-desync=fake'])).toEqual({
      ok: false,
      reason: 'profile-limit',
    });
  });

  it('refuses a copy that exceeds the argument limit', () => {
    const draft = [Array.from({ length: 8190 }, () => '--flag')];

    expect(copyProfileIntoDraft(draft, ['--one', '--two', '--three'])).toEqual({
      ok: false,
      reason: 'argument-limit',
    });
  });
});
