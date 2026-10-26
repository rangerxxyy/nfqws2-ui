export function isValidNetworkInterfaceName(name: string): boolean {
  return name !== '.' && name !== '..' && /^[A-Za-z0-9_.@+-]{1,15}$/.test(name);
}
