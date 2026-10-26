import { describe, expect, it } from 'vitest';
import { diagnoseProfileLuaArguments, reorderProfileSteps, replaceDesyncFunction, replaceDesyncParameters, replaceProfileArgument, replaceProfileName, serializeProfileForSharing, splitDesyncParameters, updateDesyncParameter, removeDesyncParameter, appendDesyncParameter, parseProfilePaste } from './profile-editor';

describe('reorderProfileSteps', () => {
  it('moves ordered payload and desync steps while leaving filters in place', () => {
    const args = ['--filter-tcp=443', '--lua-desync=fake', '--payload=tls_client_hello', '--lua-desync=multisplit:pos=1'];

    expect(reorderProfileSteps(args, 0, 1)).toEqual([
      '--filter-tcp=443',
      '--payload=tls_client_hello',
      '--lua-desync=fake',
      '--lua-desync=multisplit:pos=1',
    ]);
    expect(args[1]).toBe('--lua-desync=fake');
  });

  it('keeps packet ranges in the same ordered sequence as payload and Lua steps', () => {
    expect(reorderProfileSteps(['--filter-tcp=443', '--payload=tls_client_hello', '--out-range=1', '--lua-desync=fake'], 1, 1))
      .toEqual(['--filter-tcp=443', '--payload=tls_client_hello', '--lua-desync=fake', '--out-range=1']);
  });

  it('keeps the order unchanged when the requested step has no neighbour', () => {
    const args = ['--lua-desync=fake', '--lua-desync=multisplit'];

    expect(reorderProfileSteps(args, 0, -1)).toEqual(args);
  });
});

describe('desync step editing', () => {
  it('reports missing required Lua arguments and unavailable functions from the installed catalog', () => {
    const catalog = [{ name: 'multisplit', file: 'zapret-auto.lua', docs: [], arguments: ['pos — split positions'], required: ['pos'] }];
    expect(diagnoseProfileLuaArguments(['--lua-desync=multisplit', '--lua-desync=ghost', '--lua-desync=multisplit:pos=1'], catalog)).toEqual([
      { argument: '--lua-desync=multisplit', message: 'Для multisplit не указан обязательный параметр pos' },
      { argument: '--lua-desync=ghost', message: 'Функция ghost отсутствует в установленных Lua-скриптах' },
    ]);
  });

  it('does not guess warnings when the installed Lua catalog could not be loaded', () => {
    expect(diagnoseProfileLuaArguments(['--lua-desync=unknown'], [])).toEqual([]);
  });
  it('changes a step function without losing its parameters', () => {
    expect(replaceDesyncFunction('--lua-desync=fake:blob=tls_clienthello:tcp_md5', 'multisplit'))
      .toBe('--lua-desync=multisplit:blob=tls_clienthello:tcp_md5');
  });

  it('changes step parameters while retaining the selected function', () => {
    expect(replaceDesyncParameters('--lua-desync=multisplit:pos=1', 'pos=2:midsld'))
      .toBe('--lua-desync=multisplit:pos=2:midsld');
  });

  it('edits individual key-value parameters, flags and additions without changing the function', () => {
    const argument = '--lua-desync=hostfakesplit:host=google.com:tcp_md5';
    expect(splitDesyncParameters(argument)).toEqual(['host=google.com', 'tcp_md5']);
    expect(updateDesyncParameter(argument, 0, 'host=example.org')).toBe('--lua-desync=hostfakesplit:host=example.org:tcp_md5');
    expect(removeDesyncParameter(argument, 1)).toBe('--lua-desync=hostfakesplit:host=google.com');
    expect(appendDesyncParameter(argument, 'repeats=3')).toBe('--lua-desync=hostfakesplit:host=google.com:tcp_md5:repeats=3');
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

describe('profile sharing', () => {
  it('creates a chat-friendly header with the exact ordered arguments', () => {
    expect(serializeProfileForSharing('HTTP/HTTPS по сайтам', ['--filter-tcp=443', '--payload=tls_client_hello', '--lua-desync=fake'], '1.23.1'))
      .toBe('# nfqws2 v1.23.1 · HTTP/HTTPS по сайтам\n--filter-tcp=443 --payload=tls_client_hello --lua-desync=fake');
  });
});

describe('profile paste parsing', () => {
  it('accepts quoted config lines, comments, continuations, Lua shorthand and named profiles', () => {
    const pasted = '# profile from chat\nNFQWS_ARGS_CUSTOM="--filter-tcp=443 \\\n      --lua-desync=hostfakesplit:host=example.com"\n--new=video --filter-tcp=80 fake:blob=tls_clienthello';
    expect(parseProfilePaste(pasted)).toEqual({
      ok: true,
      profiles: [
        ['--filter-tcp=443', '--lua-desync=hostfakesplit:host=example.com'],
        ['--name=video', '--filter-tcp=80', '--lua-desync=fake:blob=tls_clienthello'],
      ],
    });
  });

  it('rejects shell operators and expansion syntax even when quoted', () => {
    expect(parseProfilePaste('--filter-tcp=443; reboot')).toMatchObject({ ok: false, reason: 'shell-syntax' });
    expect(parseProfilePaste('"$(reboot)"')).toMatchObject({ ok: false, reason: 'shell-syntax' });
  });
});
