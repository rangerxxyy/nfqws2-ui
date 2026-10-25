import { describe, expect, it } from 'vitest';
import { strategyNeedsWindowsTcpTimestamps } from './strategy-capability';

describe('strategyNeedsWindowsTcpTimestamps', () => {
  it('flags a tcp_ts step when no other fooling method is present', () => {
    expect(strategyNeedsWindowsTcpTimestamps([
      '--lua-desync=fake:tcp_ts=-600000',
    ])).toBe(true);
  });

  it('does not flag a tcp_ts step that also has another fooling method in the same step', () => {
    expect(strategyNeedsWindowsTcpTimestamps([
      '--lua-desync=fake:tcp_ts=-600000:tcp_md5',
    ])).toBe(false);
    expect(strategyNeedsWindowsTcpTimestamps([
      '--lua-desync=fake:tcp_ts=-600000',
      '--lua-desync=multisplit:pos=1',
    ])).toBe(true);
  });

  it('does not treat tcp_ts_up or strings in unrelated parameters as tcp_ts', () => {
    expect(strategyNeedsWindowsTcpTimestamps(['--lua-desync=fake:tcp_ts_up'])).toBe(false);
    expect(strategyNeedsWindowsTcpTimestamps(['--lua-desync=fake:host=tcp_ts=1'])).toBe(false);
  });
});
