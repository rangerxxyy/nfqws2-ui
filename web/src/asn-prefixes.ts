export function parseAsnInput(input: string): number | null {
  const match = input.trim().match(/^(?:AS)?([0-9]{1,10})$/i);
  if (!match) return null;
  const asn = Number(match[1]);
  return Number.isSafeInteger(asn) && asn > 0 && asn <= 0xffff_ffff ? asn : null;
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
