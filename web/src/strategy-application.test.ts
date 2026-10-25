import { describe, expect, it } from 'vitest';
import { replaceBaseDesyncSteps } from './strategy-application';

describe('replaceBaseDesyncSteps', () => {
  it('replaces existing desync steps and preserves other base arguments', () => {
    expect(replaceBaseDesyncSteps('--qnum=200 --lua-desync=old:pos=1 --filter-tcp=443', [
      '--lua-desync=multisplit:pos=2',
      '--lua-desync=fakedsplit:pos=1',
    ])).toBe('--qnum=200 --filter-tcp=443 --lua-desync=multisplit:pos=2 --lua-desync=fakedsplit:pos=1');
  });

  it('rejects empty candidates and values that are not plain lua desync arguments', () => {
    expect(() => replaceBaseDesyncSteps('--qnum=200', [])).toThrow('Параметры кандидата недоступны');
    expect(() => replaceBaseDesyncSteps('--qnum=200', ['--filter-tcp=443'])).toThrow('Разрешены только параметры --lua-desync');
    expect(() => replaceBaseDesyncSteps('--qnum=200', ['--lua-desync=multi split'])).toThrow('Разрешены только параметры --lua-desync');
  });
});
