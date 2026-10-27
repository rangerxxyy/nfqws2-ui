export function isValidNetworkInterfaceName(name: string): boolean {
  return name !== '.' && name !== '..' && /^[A-Za-z0-9_.@+-]{1,15}$/.test(name);
}

const preferredNetworkInterfaces = ['eth0', 'eth1', 'eth2', 'eth3', 'eth4', 'ppp0', 'pppoe-wan', 'wan', 'wan0'];

export function orderNetworkInterfaces(names: string[], configured = ''): string[] {
  const remaining = new Set(names.filter(isValidNetworkInterfaceName));
  const ordered: string[] = [];
  for (const name of preferredNetworkInterfaces) {
    if (remaining.delete(name)) ordered.push(name);
  }
  if (isValidNetworkInterfaceName(configured) && !ordered.includes(configured)) {
    remaining.delete(configured);
    ordered.push(configured);
  }
  return [...ordered, ...[...remaining].sort((left, right) => left.localeCompare(right, 'en', { numeric: true, sensitivity: 'base' }))];
}
