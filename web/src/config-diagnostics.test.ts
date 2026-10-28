import { describe, expect, it } from 'vitest';
import { configLuaArgumentEntries } from './config-diagnostics';
import type { NfqwsLuaFunctionInfo } from './api';
import { diagnoseConfigLuaArguments } from './config-diagnostics';

const catalog: NfqwsLuaFunctionInfo[] = [{ name: 'fake', file: 'zapret-auto.lua', docs: [], arguments: ['repeats — число'], fileArguments: [], standardGroups: [], required: ['blob'] }];

describe('configuration Lua diagnostics', () => {
  it('extracts desync steps from active groups and tracks physical line numbers', () => {
    const content = [
      '# --lua-desync=commented:typo=1',
      'NFQWS_ARGS="--filter-tcp=443 --lua-desync=fake:repeats=2"',
      'NFQWS_ARGS_CUSTOM="--new --lua-desync=hostfakesplit:host=example.com:typo=1"',
    ].join('\n');
    expect(configLuaArgumentEntries(content)).toEqual([
      { argument: '--lua-desync=fake:repeats=2', line: 2 },
      { argument: '--lua-desync=hostfakesplit:host=example.com:typo=1', line: 3 },
    ]);
  });

  it('handles CRLF, single-quoted arguments, and multiple steps on one line', () => {
    expect(configLuaArgumentEntries("NFQWS_ARGS='--lua-desync=fake:repeats=2 --lua-desync=hostfakesplit:typo=1'\r\n"))
      .toEqual([
        { argument: '--lua-desync=fake:repeats=2', line: 1 },
        { argument: '--lua-desync=hostfakesplit:typo=1', line: 1 },
      ]);
  });

  it('returns line-numbered Lua warnings for the Apply confirmation without treating them as errors', () => {
    expect(diagnoseConfigLuaArguments('NFQWS_ARGS="--lua-desync=fake:typo=1"\n', catalog)).toEqual([
      { line: 1, argument: '--lua-desync=fake:typo=1', message: 'Для fake не указан обязательный параметр blob' },
      { line: 1, argument: '--lua-desync=fake:typo=1', message: 'Неизвестный параметр «typo»' },
    ]);
  });

  it('ignores Lua-looking text after an inline comment marker', () => {
    expect(configLuaArgumentEntries('NFQWS_ARGS="--lua-desync=fake:repeats=2 # --lua-desync=not-real:typo=1"\n'))
      .toEqual([{ argument: '--lua-desync=fake:repeats=2', line: 1 }]);
  });

  it('keeps warnings empty when Lua catalog is unavailable or content only contains dynamic mode references', () => {
    expect(diagnoseConfigLuaArguments('NFQWS_ARGS="$MODE_LIST"\n', catalog)).toEqual([]);
    expect(diagnoseConfigLuaArguments('NFQWS_ARGS="--lua-desync=fake:typo=1"\n', [])).toEqual([]);
  });
});
