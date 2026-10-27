import { describe, expect, it } from 'vitest';
import { configLuaArgumentEntries } from './config-diagnostics';

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
});
