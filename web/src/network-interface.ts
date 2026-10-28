export function isValidNetworkInterfaceName(name: string): boolean {
  return name !== '.' && name !== '..' && /^[A-Za-z0-9_.@+-]{1,15}$/.test(name);
}

export function isValidNetworkInterfaceSelection(value: string): boolean {
  const names = value.trim().split(/\s+/);
  return names.length > 0 && names.length <= 16 && names.every(isValidNetworkInterfaceName);
}

const preferredNetworkInterfaces = ['eth0', 'eth1', 'eth2', 'eth3', 'eth4', 'ppp0', 'pppoe-wan', 'wan', 'wan0'];

export function orderNetworkInterfaces(names: string[], configured = '', defaults: string[] = []): string[] {
  const remaining = new Set(names.filter(isValidNetworkInterfaceName));
  const ordered: string[] = [];
  for (const name of configured.trim().split(/\s+/)) {
    if (!isValidNetworkInterfaceName(name) || ordered.includes(name)) continue;
    remaining.delete(name);
    ordered.push(name);
  }
  for (const name of defaults) {
    if (isValidNetworkInterfaceName(name) && !ordered.includes(name)) {
      remaining.delete(name);
      ordered.push(name);
    }
  }
  for (const name of preferredNetworkInterfaces) {
    if (remaining.delete(name)) ordered.push(name);
  }
  return [...ordered, ...[...remaining].sort((left, right) => left.localeCompare(right, 'en', { numeric: true, sensitivity: 'base' }))];
}
