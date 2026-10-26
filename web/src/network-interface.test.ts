import { describe, expect, it } from 'vitest';
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
});
