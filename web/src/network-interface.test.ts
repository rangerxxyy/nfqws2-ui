import { describe, expect, it } from 'vitest';
import * as networkInterface from './network-interface';
import { isValidNetworkInterfaceName } from './network-interface';

describe('network interface names', () => {
  it('accepts common Linux interface names', () => {
    expect(isValidNetworkInterfaceName('eth0')).toBe(true);
    expect(isValidNetworkInterfaceName('br-lan')).toBe(true);
    expect(isValidNetworkInterfaceName('wlan0.1')).toBe(true);
  });

  it('rejects empty, overlong, whitespace, path, shell, and special dot names', () => {
    for (const name of ['', 'abcdefghijklmnop', 'eth 0', '../eth0', '$(id)', '.', '..']) {
      expect(isValidNetworkInterfaceName(name), name).toBe(false);
    }
  });

  it('puts the configured interface and detected common uplinks first without inventing options', () => {
    const order = (networkInterface as unknown as { orderNetworkInterfaces?: (names: string[], configured?: string) => string[] }).orderNetworkInterfaces;
    expect(order).toBeTypeOf('function');
    expect(order?.(['wlan0', 'br-lan', 'eth3', 'wg0', 'ppp0', 'eth0', 'eth1'], 'eth3'))
      .toEqual(['eth0', 'eth1', 'eth3', 'ppp0', 'br-lan', 'wg0', 'wlan0']);
    expect(order?.([], 'eth3')).toEqual(['eth3']);
  });
});
