import { describe, expect, it } from 'vitest';
import { mergeAsnPrefixes, parseAsnInput } from './asn-prefixes';

describe('ASN prefix input', () => {
  it('accepts decimal ASNs with or without the AS prefix', () => {
    expect(parseAsnInput('AS15169')).toBe(15169);
    expect(parseAsnInput(' 13335 ')).toBe(13335);
  });

  it('rejects malformed and out-of-range ASNs', () => {
    for (const value of ['', 'AS', 'AS0', '-1', '1.2', '4294967296', 'AS1 extra']) {
      expect(parseAsnInput(value)).toBeNull();
    }
  });

  it('merges unique prefixes into existing list content without changing existing lines', () => {
    expect(mergeAsnPrefixes('# manually managed\n192.0.2.0/24\n', ['192.0.2.0/24', '198.51.100.0/24', '198.51.100.0/24'])).toEqual({
      content: '# manually managed\n192.0.2.0/24\n198.51.100.0/24\n',
      added: 1,
      alreadyPresent: 1,
    });
  });
});
