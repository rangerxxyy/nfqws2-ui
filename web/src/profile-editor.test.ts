import { describe, expect, it } from 'vitest';
import { reorderProfileSteps, replaceDesyncFunction, replaceDesyncParameters, replaceProfileArgument, replaceProfileName } from './profile-editor';

describe('reorderProfileSteps', () => {
  it('moves a desync step while leaving filters and payload arguments in place', () => {
    const args = ['--filter-tcp=443', '--lua-desync=fake', '--payload=tls_client_hello', '--lua-desync=multisplit:pos=1'];

    expect(reorderProfileSteps(args, 0, 1)).toEqual([
      '--filter-tcp=443',
      '--lua-desync=multisplit:pos=1',
      '--payload=tls_client_hello',
      '--lua-desync=fake',
    ]);
    expect(args[1]).toBe('--lua-desync=fake');
  });

  it('keeps the order unchanged when the requested step has no neighbour', () => {
    const args = ['--lua-desync=fake', '--lua-desync=multisplit'];

    expect(reorderProfileSteps(args, 0, -1)).toEqual(args);
  });
});

describe('desync step editing', () => {
  it('changes a step function without losing its parameters', () => {
    expect(replaceDesyncFunction('--lua-desync=fake:blob=tls_clienthello:tcp_md5', 'multisplit'))
      .toBe('--lua-desync=multisplit:blob=tls_clienthello:tcp_md5');
  });

  it('changes step parameters while retaining the selected function', () => {
    expect(replaceDesyncParameters('--lua-desync=multisplit:pos=1', 'pos=2:midsld'))
      .toBe('--lua-desync=multisplit:pos=2:midsld');
  });
});

describe('profile fields', () => {
  it('renames a profile without changing its other arguments', () => {
    expect(replaceProfileName(['--filter-tcp=443', '--lua-desync=fake'], 'gaming_profile'))
      .toEqual(['--name=gaming_profile', '--filter-tcp=443', '--lua-desync=fake']);
  });

  it('updates one argument and rejects a value that could become multiple arguments', () => {
    expect(replaceProfileArgument(['--filter-tcp=443', '--lua-desync=fake'], 0, '--filter-tcp=80,443'))
      .toEqual(['--filter-tcp=80,443', '--lua-desync=fake']);
    expect(replaceProfileArgument(['--filter-tcp=443'], 0, '--filter-tcp=80 --new'))
      .toEqual(['--filter-tcp=443']);
  });

  it('removes a blank custom name and rejects punctuation outside the backend rules', () => {
    expect(replaceProfileName(['--name=old', '--filter-tcp=443'], ''))
      .toEqual(['--filter-tcp=443']);
    expect(replaceProfileName(['--filter-tcp=443'], 'bad name'))
      .toEqual(['--filter-tcp=443']);
  });
});
