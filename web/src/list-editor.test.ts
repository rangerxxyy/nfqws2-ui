import { describe, expect, it } from 'vitest';
import { deduplicateListContent } from './list-editor';

describe('list editor helpers', () => {
  it('removes case-insensitive domain duplicates while preserving comments, order, whitespace, and CRLF', () => {
    const source = '# hand-written list\r\n  Example.org  \r\n\r\nexample.org.\r\n*.CDN.example.org\r\n*.cdn.example.org\r\n# tail\r\n';
    const result = deduplicateListContent(source, 'user.list');

    expect(result).toEqual({
      content: '# hand-written list\r\n  Example.org  \r\n\r\n*.CDN.example.org\r\n# tail\r\n',
      removed: 2,
    });
  });

  it('deduplicates IP entries case-insensitively but preserves distinct CIDRs', () => {
    const source = '2001:0DB8:0000:0000:0000:0000:0000:0001/64\n2001:db8::1/64\n192.0.2.0/24\n192.0.2.0/25\n';
    expect(deduplicateListContent(source, 'ipset.list')).toEqual({
      content: '2001:0DB8:0000:0000:0000:0000:0000:0001/64\n192.0.2.0/24\n192.0.2.0/25\n',
      removed: 1,
    });
  });

  it('leaves a list byte-for-byte unchanged when there are no duplicates', () => {
    const source = '# note\nexample.org\n';
    expect(deduplicateListContent(source, 'user.list')).toEqual({ content: source, removed: 0 });
  });
});
