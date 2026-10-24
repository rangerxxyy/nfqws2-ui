import { describe, expect, it } from 'vitest';
import { parseCustomStrategies } from './custom-strategy-input';

describe('custom strategy input', () => {
  it('keeps several desync steps together when pasted on separate lines', () => {
    expect(parseCustomStrategies('--lua-desync=fake:blob=tls_clienthello\n--lua-desync=multisplit:pos=1'))
      .toEqual([['--lua-desync=fake:blob=tls_clienthello', '--lua-desync=multisplit:pos=1']]);
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
