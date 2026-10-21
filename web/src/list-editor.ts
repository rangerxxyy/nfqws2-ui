import type { NfqwsListId } from './api';

export function deduplicateListContent(content: string, id: NfqwsListId): { content: string; removed: number } {
  const newline = content.includes('\r\n') ? '\r\n' : '\n';
  const seen = new Set<string>();
  const output: string[] = [];
  let removed = 0;
  for (const line of content.split(/\r\n|\n/)) {
    const value = line.trim();
    if (!value || value.startsWith('#')) {
      output.push(line);
      continue;
    }
    const key = listEntryKey(value, id);
    if (seen.has(key)) {
      removed += 1;
      continue;
    }
    seen.add(key);
    output.push(line);
  }
  return removed ? { content: output.join(newline), removed } : { content, removed: 0 };
}

function listEntryKey(value: string, id: NfqwsListId): string {
  let key = value.toLocaleLowerCase('en-US');
  if (!id.startsWith('ipset')) return key.replace(/\.$/, '');
  const [address, prefix, ...extra] = key.split('/');
  if (extra.length) return key;
  let normalizedAddress = address;
  if (address.includes(':')) {
    try { normalizedAddress = new URL(`http://[${address}]/`).hostname.slice(1, -1); }
    catch { return key; }
  }
  return prefix === undefined ? normalizedAddress : `${normalizedAddress}/${prefix}`;
}
