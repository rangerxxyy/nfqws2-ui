import { describe, expect, it } from 'vitest';
import { configLuaArgumentEntries } from './config-diagnostics';
import type { NfqwsLuaFunctionInfo } from './api';
import { diagnoseConfigLuaArguments } from './config-diagnostics';

const catalog: NfqwsLuaFunctionInfo[] = [{ name: 'fake', file: 'zapret-auto.lua', docs: [], arguments: ['repeats — число', 'blob — имя блоба'], fileArguments: [], standardGroups: [], required: ['blob'] }];

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
    expect(diagnoseConfigLuaArguments('NFQWS_BASE_ARGS="--lua-init=@zapret-auto.lua"\nNFQWS_ARGS="--lua-desync=fake:typo=1"\n', catalog)).toEqual([
      { line: 2, argument: '--lua-desync=fake:typo=1', message: 'Для fake не указан обязательный параметр blob' },
      { line: 2, argument: '--lua-desync=fake:typo=1', message: 'Неизвестный параметр «typo»' },
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

  it('warns when a configured Lua function comes from a script not loaded by NFQWS_BASE_ARGS', () => {
    expect(diagnoseConfigLuaArguments('NFQWS_BASE_ARGS="--filter-tcp=443"\nNFQWS_ARGS="--lua-desync=fake:blob=0x00000000"\n', catalog))
      .toEqual([{ line: 2, argument: '--lua-desync=fake:blob=0x00000000', message: 'Файл zapret-auto.lua для функции fake не подключён через --lua-init' }]);
  });

  it('matches loaded Lua scripts by basename and skips the load check for unresolved base arguments', () => {
    expect(diagnoseConfigLuaArguments('NFQWS_BASE_ARGS="--lua-init=@/opt/etc/nfqws2/lua/zapret-auto.lua"\nNFQWS_ARGS="--lua-desync=fake:blob=0x00000000"\n', catalog)).toEqual([]);
    expect(diagnoseConfigLuaArguments('NFQWS_BASE_ARGS="$NFQWS_BASE_ARGS"\nNFQWS_ARGS="--lua-desync=fake:blob=0x00000000"\n', catalog)).toEqual([]);
  });
});
