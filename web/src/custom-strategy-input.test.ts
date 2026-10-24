import { describe, expect, it } from 'vitest';
import { parseCustomStrategies } from './custom-strategy-input';

describe('custom strategy input', () => {
  it('reads the four strategies from the reported example as four separate candidates', () => {
    const input = [
      '--lua-desync=hostfakesplit:host=google.com:midhost=host-2:altorder=1:seqovl=sniext+3:seqovl_pattern=tls_clienthello:badsum:tcp_md5:tcp_ts_up:disorder_after',
      '--lua-desync=multisplit:pos=sniext:seqovl=10:seqovl_pattern=tls_clienthello:badsum:tcp_md5:tcp_ts_up:disorder_after',
      '--lua-desync=fakedsplit:oob:oob_data=0x00:pos=midsld:pattern=0x0F0F0F0F:tcp_seq=0:tcp_ack=-1000:ip_autottl=-2,3-20',
      '--lua-desync=hostfakesplit:host=google.com:midhost=host-2:seqovl=sniext+3:seqovl_pattern=tls_clienthello:badsum:tcp_md5',
    ].join('\n');
    expect(parseCustomStrategies(input)).toHaveLength(4);
    expect(parseCustomStrategies(input).map(([step]) => step.split('=')[1].split(':')[0]))
      .toEqual(['hostfakesplit', 'multisplit', 'fakedsplit', 'hostfakesplit']);
  });

  it('treats each pasted line as a separate strategy', () => {
    expect(parseCustomStrategies('--lua-desync=fake:blob=tls_clienthello\n--lua-desync=multisplit:pos=1'))
      .toEqual([['--lua-desync=fake:blob=tls_clienthello'], ['--lua-desync=multisplit:pos=1']]);
  });

  it('ignores full-line comments from pasted profiles', () => {
    expect(parseCustomStrategies('# --lua-desync=multisplit:pos=0\n--lua-desync=fakedsplit:pos=1'))
      .toEqual([['--lua-desync=fakedsplit:pos=1']]);
  });

  it('accepts a bare Lua desync function copied from strategy notes', () => {
    expect(parseCustomStrategies('multisplit:pos=1,midsld'))
      .toEqual([['--lua-desync=multisplit:pos=1,midsld']]);
  });

  it('keeps multiple steps on one line together and supports explicit continuations', () => {
    expect(parseCustomStrategies('--lua-desync=fake:blob=tls_clienthello --lua-desync=multisplit:pos=1\n--lua-desync=fakedsplit:pos=2 \\\n--lua-desync=hostfakesplit:host=example.com'))
      .toEqual([
        ['--lua-desync=fake:blob=tls_clienthello', '--lua-desync=multisplit:pos=1'],
        ['--lua-desync=fakedsplit:pos=2', '--lua-desync=hostfakesplit:host=example.com'],
      ]);
  });

  it('splits pasted profiles at --new, keeps them in order, and ignores unrelated arguments', () => {
    expect(parseCustomStrategies('--name=first --lua-desync="fake:blob=tls_clienthello" --lua-desync=multisplit:pos=1 --new --filter-tcp=443 --lua-desync=fakedsplit:pos=1'))
      .toEqual([
        ['--lua-desync=fake:blob=tls_clienthello', '--lua-desync=multisplit:pos=1'],
        ['--lua-desync=fakedsplit:pos=1'],
      ]);
  });

  it('rejects more than ten pasted candidate strategies', () => {
    const input = Array.from({ length: 11 }, (_, index) => `--lua-desync=multisplit:pos=${index + 1}`).join(' --new ');
    expect(() => parseCustomStrategies(input)).toThrow('не более 10');
  });
});
