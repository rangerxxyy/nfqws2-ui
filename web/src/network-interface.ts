export function isValidNetworkInterfaceName(name: string): boolean {
  return name !== '.' && name !== '..' && /^[A-Za-z0-9_.@+-]{1,15}$/.test(name);
}

export function isValidNetworkInterfaceSelection(value: string): boolean {
  const names = value.trim().split(/\s+/);
  return names.length > 0 && names.length <= 16 && names.every(isValidNetworkInterfaceName);
}

// Common device names from OpenWrt/Keenetic layouts; these are only ordered
// when the router actually reports them under /sys/class/net.
const preferredNetworkInterfaces = ['eth0', 'eth1', 'eth2', 'eth3', 'eth4', 'eth0.2', 'eth1.2', 'eth2.2', 'ppp0', 'ppp1', 'pppoe0', 'pppoe-wan', 'wan', 'wan0', 'wwan0', 'usb0', 'br-wan'];

export function orderNetworkInterfaces(names: string[], configured = '', defaults: string[] = []): string[] {
  const remaining = new Set(names.filter((name) => name !== 'lo' && isValidNetworkInterfaceName(name)));
  const ordered: string[] = [];
  for (const name of defaults) {
    if (name !== 'lo' && isValidNetworkInterfaceName(name) && !ordered.includes(name)) {
      remaining.delete(name);
      ordered.push(name);
    }
  }
  for (const name of configured.trim().split(/\s+/)) {
    if (!isValidNetworkInterfaceName(name) || ordered.includes(name)) continue;
    remaining.delete(name);
    ordered.push(name);
  }
  for (const name of preferredNetworkInterfaces) {
    if (remaining.delete(name)) ordered.push(name);
  }
  return [...ordered, ...[...remaining].sort((left, right) => left.localeCompare(right, 'en', { numeric: true, sensitivity: 'base' }))];
}
