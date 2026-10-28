import { describe, expect, it } from 'vitest';
import * as networkInterface from './network-interface';
import { isValidNetworkInterfaceName, isValidNetworkInterfaceSelection } from './network-interface';

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

  it('accepts multiple safe interface names separated by spaces but rejects invalid entries', () => {
    expect(isValidNetworkInterfaceSelection('eth0 pppoe-wan')).toBe(true);
    expect(isValidNetworkInterfaceSelection(' eth0   eth1 ')).toBe(true);
    expect(isValidNetworkInterfaceSelection('')).toBe(false);
    expect(isValidNetworkInterfaceSelection('eth0 $(id)')).toBe(false);
    expect(isValidNetworkInterfaceSelection(Array.from({ length: 17 }, (_, index) => `eth${index}`).join(' '))).toBe(false);
  });

  it('puts detected default-route uplinks before the current interface without inventing options', () => {
    const order = (networkInterface as unknown as { orderNetworkInterfaces?: (names: string[], configured?: string, defaults?: string[]) => string[] }).orderNetworkInterfaces;
    expect(order).toBeTypeOf('function');
    expect(order?.(['wlan0', 'br-lan', 'eth3', 'wg0', 'ppp0', 'eth0', 'eth1'], 'eth3', ['ppp0']))
      .toEqual(['ppp0', 'eth3', 'eth0', 'eth1', 'br-lan', 'wg0', 'wlan0']);
    expect(order?.(['eth0', 'eth3', 'wan'], '', ['eth3']))
      .toEqual(['eth3', 'eth0', 'wan']);
    expect(order?.(['eth0', 'eth3', 'ppp0'], 'eth3 ppp0', ['eth0']))
      .toEqual(['eth0', 'eth3', 'ppp0']);
    expect(order?.([], 'eth3')).toEqual(['eth3']);
  });

  it('does not offer loopback as a provider interface or mark it as an internet route', () => {
    const order = (networkInterface as unknown as { orderNetworkInterfaces?: (names: string[], configured?: string, defaults?: string[]) => string[] }).orderNetworkInterfaces;
    expect(order?.(['lo', 'eth3', 'pppoe-wan'], '', ['lo', 'eth3'])).toEqual(['eth3', 'pppoe-wan']);
    expect(order?.(['lo', 'eth3'], 'lo', ['lo', 'eth3'])).toEqual(['eth3', 'lo']);
  });
});
