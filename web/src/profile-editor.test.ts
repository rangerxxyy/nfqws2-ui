import { describe, expect, it } from 'vitest';
import { profileArgumentChoices, profileOrderWarnings, summarizeProfileSelection, diagnoseProfileLuaArguments, getLuaProfileHelp, getLuaProfileParameterOptions, moveProfileStepTo, parseLuaParameterOption, reorderProfileSteps, replaceDesyncFunction, replaceDesyncParameters, replaceProfileArgument, replaceProfileName, serializeProfileForSharing, splitDesyncParameters, updateDesyncParameter, removeDesyncParameter, appendDesyncParameter, parseProfilePaste } from './profile-editor';

describe('profile builder choices', () => {
  it('offers editable match conditions, list scopes, and ordered strategy steps', () => {
    const choices = profileArgumentChoices([], [{ id: 'youtube', path: '/opt/etc/nfqws2/lists/youtube.txt', readable: true, writable: true }]);
    expect(choices.map((choice) => choice.value)).toEqual(expect.arrayContaining([
      '--filter-tcp=443', '--filter-udp=443', '--filter-l7=tls', '--hostlist=/opt/etc/nfqws2/lists/youtube.txt',
      '--hostlist-exclude-domains=example.com', '--hostlist-domains=example.com', '--ipset-ip=192.0.2.1',
      '--payload=tls_client_hello', '--in-range=1', '--out-range=1', '--lua-desync=fake',
    ]));
  });

  it('keeps available list choices aligned with current profile list arguments', () => {
    const choices = profileArgumentChoices(['--hostlist=/opt/etc/nfqws2/lists/youtube.txt'], [{ id: 'youtube-ipset', path: '/opt/etc/nfqws2/lists/youtube-ipset.txt', readable: true, writable: true }]);
    expect(choices.some((choice) => choice.value.startsWith('--hostlist='))).toBe(false);
    expect(choices.some((choice) => choice.value.startsWith('--ipset='))).toBe(true);
  });

  it('offers auto host lists only from the router list inventory', () => {
    const absent = profileArgumentChoices([], []);
    const installed = profileArgumentChoices([], [{ id: 'auto.list', path: '/etc/nfqws2/lists/auto.list', readable: true, writable: true }]);
    expect(absent.some((choice) => choice.value.startsWith('--hostlist-auto='))).toBe(false);
    expect(installed.some((choice) => choice.value === '--hostlist-auto=/etc/nfqws2/lists/auto.list')).toBe(true);
  });
});

describe('Lua profile parameter choices', () => {
  it('combines function, file, and standard-group parameters without duplicate keys', () => {
    const functionInfo = { name: 'fake', file: 'zapret.lua', docs: [], arguments: ['blob=<name>', 'repeats=2'], fileArguments: ['detect_host — hostname source'], standardGroups: ['general'], required: ['blob'] };
    const groups = [{ name: 'general', arguments: ['tcp_ack=-1000', 'repeats=4'] }];
    expect(getLuaProfileParameterOptions(functionInfo, groups)).toEqual(['blob=<name>', 'repeats=2', 'detect_host — hostname source', 'tcp_ack=-1000']);
  });
});

describe('profile selection summary', () => {
  it('summarizes transport, protocol, and traffic scope in the profile picker', () => {
    expect(summarizeProfileSelection(['--filter-tcp=80,443', '--filter-l7=tls,http', '--hostlist=lists/user.list', '--ipset=lists/ipset.list']))
      .toBe('TCP 80,443 · TLS,HTTP · домены + IP');
  });

  it('describes an unfiltered profile without implying that it is disabled', () => {
    expect(summarizeProfileSelection(['--lua-desync=fake'])).toBe('весь трафик · без списков');
    expect(summarizeProfileSelection(['--hostlist-exclude=lists/excluded.list'])).toBe('весь трафик · все, кроме исключений');
  });
});

describe('profile order warnings', () => {
  it('warns when an earlier unscoped rule overlaps a later profile', () => {
    expect(profileOrderWarnings([
      ['--filter-tcp=80,443', '--lua-desync=fake'],
      ['--filter-tcp=443', '--filter-l7=tls', '--lua-desync=split'],
    ])).toEqual([[], ['Трафик может быть обработан профилем #1 раньше этого профиля.']]);
  });

  it('does not warn for disjoint ports or an earlier profile scoped to a host list', () => {
    expect(profileOrderWarnings([['--filter-tcp=80'], ['--filter-tcp=443']])).toEqual([[], []]);
    expect(profileOrderWarnings([['--hostlist=lists/video.list', '--filter-tcp=443'], ['--filter-tcp=443']])).toEqual([[], []]);
  });

  it('does not treat an omitted transport as unrestricted when another transport filter is set', () => {
    expect(profileOrderWarnings([['--filter-tcp=80'], ['--filter-udp=443']])).toEqual([[], []]);
  });
});

describe('reorderProfileSteps', () => {
  it('moves a dragged step across several ordered steps without shifting unrelated arguments', () => {
    const args = ['--filter-tcp=443', '--lua-desync=fake', '--new-flag', '--payload=tls_client_hello', '--lua-desync=multisplit:pos=1'];
    expect(moveProfileStepTo(args, 0, 2)).toEqual(['--filter-tcp=443', '--payload=tls_client_hello', '--new-flag', '--lua-desync=multisplit:pos=1', '--lua-desync=fake']);
    expect(moveProfileStepTo(args, 2, 1)).toEqual(['--filter-tcp=443', '--lua-desync=fake', '--new-flag', '--lua-desync=multisplit:pos=1', '--payload=tls_client_hello']);
  });
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
  it('collects complete installed Lua function help and referenced standard groups', () => {
    const functionInfo = { name: 'fake', file: 'zapret-antidpi.lua', docs: ['Creates a fake packet.', 'The second help line.'], arguments: ['blob=<name> — fake data', 'repeats=2 — count'], standardGroups: ['general'], required: ['blob'] };
    const groups = [{ name: 'general', arguments: ['tcp_ack=-1000 — ACK offset', 'repeats=4 — default repeats'] }, { name: 'other', arguments: ['unused=1'] }];

    expect(getLuaProfileHelp(functionInfo, groups)).toEqual({
      docs: ['Creates a fake packet.', 'The second help line.'],
      arguments: ['blob=<name> — fake data', 'repeats=2 — count'],
      fileArguments: [],
      standardGroups: [{ name: 'general', arguments: ['tcp_ack=-1000 — ACK offset', 'repeats=4 — default repeats'] }],
      required: ['blob'],
    });
  });

  it('turns documented Lua parameter entries into editable flag or key-value options', () => {
    expect(parseLuaParameterOption('tcp_ack=-1000 — acknowledgement offset')).toEqual({ key: 'tcp_ack', value: '-1000', display: 'tcp_ack=-1000 — acknowledgement offset' });
    expect(parseLuaParameterOption('blob=<name> — select a blob')).toEqual({ key: 'blob', value: '', display: 'blob=<name> — select a blob' });
    expect(parseLuaParameterOption('bad key')).toBeNull();
  });
  it('offers nfqws2 compatibility parameters when older Lua comments omit them', () => {
    const info = { name: 'fakedsplit', file: 'zapret-auto.lua', docs: [], arguments: ['pos=<marker>'], fileArguments: [], standardGroups: [], required: [] };
    expect(getLuaProfileParameterOptions(info, [])).toEqual(expect.arrayContaining([
      'altorder=1 — поддерживается nfqws2', 'oob — поддерживается nfqws2',
      'oob_data=0x00 — поддерживается nfqws2', 'nofake2 — поддерживается nfqws2',
    ]));
  });
  it('reports missing required Lua arguments and unavailable functions from the installed catalog', () => {
    const catalog = [{ name: 'multisplit', file: 'zapret-auto.lua', docs: [], arguments: ['pos — split positions'], standardGroups: ['general'], required: ['pos'] }];
    expect(diagnoseProfileLuaArguments(['--lua-desync=multisplit', '--lua-desync=ghost', '--lua-desync=multisplit:pos=1'], catalog)).toEqual([
      { argument: '--lua-desync=multisplit', message: 'Для multisplit не указан обязательный параметр pos' },
      { argument: '--lua-desync=ghost', message: 'Функция ghost отсутствует в установленных Lua-скриптах' },
    ]);
  });
  it('recognizes standard and Lua-file arguments and flags undocumented step keys', () => {
    const catalog = [{ name: 'multisplit', file: 'zapret-auto.lua', docs: [], arguments: ['pos — split positions'], fileArguments: ['repeats=2 — repeat count'], standardGroups: ['general'], required: [] }];
    const groups = [{ name: 'general', arguments: ['tcp_ack=-1000 — ACK offset'] }];

    expect(diagnoseProfileLuaArguments(['--lua-desync=multisplit:pos=1:repeats=2:tcp_ack=-1:typo=3:strategy=2'], catalog, groups)).toEqual([
      { argument: '--lua-desync=multisplit:pos=1:repeats=2:tcp_ack=-1:typo=3:strategy=2', message: 'Неизвестный параметр «typo»' },
    ]);
  });

  it('accepts nfqws2 desync options that are not fully described by the installed Lua comments', () => {
    const catalog = [
      { name: 'fakedsplit', file: 'zapret-auto.lua', docs: [], arguments: ['host=<name>'], fileArguments: [], standardGroups: [], required: [] },
      { name: 'hostfakesplit', file: 'zapret-auto.lua', docs: [], arguments: ['host=<name>'], fileArguments: [], standardGroups: [], required: [] },
    ];
    expect(diagnoseProfileLuaArguments([
      '--lua-desync=fakedsplit:oob:oob_data=0x00:altorder=1',
      '--lua-desync=hostfakesplit:nofake2:altorder=1',
    ], catalog)).toEqual([]);
  });
  it('keeps compatibility options scoped to functions and continues flagging misspellings', () => {
    const catalog = [
      { name: 'fakedsplit', file: 'zapret-auto.lua', docs: [], arguments: ['pos=<marker>'], fileArguments: [], standardGroups: [], required: [] },
      { name: 'multisplit', file: 'zapret-auto.lua', docs: [], arguments: ['pos=<marker>'], fileArguments: [], standardGroups: [], required: [] },
    ];
    expect(diagnoseProfileLuaArguments(['--lua-desync=fakedsplit:typo=1', '--lua-desync=multisplit:nofake2'], catalog)).toEqual([
      { argument: '--lua-desync=fakedsplit:typo=1', message: 'Неизвестный параметр «typo»' },
      { argument: '--lua-desync=multisplit:nofake2', message: 'Параметр «nofake2» не описан для функции multisplit' },
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

  it('matches the reference profile-name limit of 32 characters', () => {
    const valid = 'a'.repeat(32);
    expect(replaceProfileName([], valid)).toEqual([`--name=${valid}`]);
    expect(replaceProfileName([], `${valid}a`)).toEqual([]);
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
