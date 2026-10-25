export function parseAsnInput(input: string): number | null {
  const match = input.trim().match(/^(?:AS)?([0-9]{1,10})$/i);
  if (!match) return null;
  const asn = Number(match[1]);
  return Number.isSafeInteger(asn) && asn > 0 && asn <= 0xffff_ffff ? asn : null;
}

export function normalizeAsnQuery(input: string): string | null {
  const query = input.trim();
  if (!query || query.length > 253 || /\s/.test(query)) return null;
  if (/^(?:AS)?[0-9]+$/i.test(query)) return parseAsnInput(query) === null ? null : query;
  const octets = query.split('.');
  if (octets.length === 4 && octets.every((part) => /^[0-9]{1,3}$/.test(part))) {
    return octets.every((part) => Number(part) <= 255) ? query : null;
  }
  const domain = query.endsWith('.') ? query.slice(0, -1) : query;
  return /^(?=.{1,253}$)(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/i.test(domain) ? query : null;
}

export function mergeAsnPrefixes(content: string, prefixes: readonly string[]) {
  const existing = new Set(content.split(/\r\n|\n/).map((line) => line.trim().toLowerCase()).filter(Boolean));
  const incoming = [...new Set(prefixes.map((prefix) => prefix.trim()).filter(Boolean))];
  const additions = incoming.filter((prefix) => !existing.has(prefix.toLowerCase()));
  const newline = content.includes('\r\n') ? '\r\n' : '\n';
  if (!additions.length) return { content, added: 0, alreadyPresent: incoming.length };
  const separator = content && !content.endsWith('\n') ? newline : '';
  return {
    content: `${content}${separator}${additions.join(newline)}${content.endsWith('\n') ? newline : ''}`,
    added: additions.length,
    alreadyPresent: incoming.length - additions.length,
  };
}
