import { describe, expect, it } from 'vitest';
import { mergeAsnPrefixes, normalizeAsnQuery, parseAsnInput } from './asn-prefixes';

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

describe('ASN, IP, and domain lookup input', () => {
  it('accepts ASN, public IP, and domain query forms', () => {
    expect(normalizeAsnQuery(' AS15169 ')).toBe('AS15169');
    expect(normalizeAsnQuery('8.8.8.8')).toBe('8.8.8.8');
    expect(normalizeAsnQuery('example.com')).toBe('example.com');
  });

  it('rejects invalid ASNs, IP addresses, and malformed domains', () => {
    for (const value of ['', 'AS0', '4294967296', '256.1.1.1', 'not a domain', 'example..com', 'bad_domain']) {
      expect(normalizeAsnQuery(value)).toBeNull();
    }
  });
});
