import { describe, expect, it } from 'vitest';
import { appendListEntries, deduplicateListContent, findNextListMatch, listIdFromTabElementId, removeListLines } from './list-editor';

describe('list editor helpers', () => {
  it('resolves each list tab from its element id so availability checks match backend ids', () => {
    expect(listIdFromTabElementId('listTab-user.list')).toBe('user.list');
    expect(listIdFromTabElementId('listTab-auto.list')).toBe('auto.list');
    expect(listIdFromTabElementId('listTab-exclude.list')).toBe('exclude.list');
    expect(listIdFromTabElementId('listTab-ipset.list')).toBe('ipset.list');
    expect(listIdFromTabElementId('listTab-ipset_exclude.list')).toBe('ipset_exclude.list');
    expect(listIdFromTabElementId('listTab-../../nfqws2.conf')).toBeNull();
  });

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

  it('appends one or more non-empty entries without changing existing line endings', () => {
    expect(appendListEntries('# note\r\nexample.org\r\n', '  one.example  \n\n two.example\n')).toBe(
      '# note\r\nexample.org\r\none.example\r\ntwo.example\r\n',
    );
    expect(appendListEntries('', '192.0.2.1\n')).toBe('192.0.2.1');
  });

  it('removes whole selected lines, including the current line when selection is empty', () => {
    const content = '# note\r\nexample.org\r\nother.example\r\n';
    expect(removeListLines(content, 8, 8)).toEqual({ content: '# note\r\nother.example\r\n', removed: 1 });
    expect(removeListLines(content, 8, 27)).toEqual({ content: '# note\r\n', removed: 2 });
  });

  it('searches case-insensitively from the caret and wraps to the first match', () => {
    expect(findNextListMatch('Alpha.test\nbeta.test\nALPHA.test', 'alpha', 5)).toEqual({ start: 21, end: 26 });
    expect(findNextListMatch('Alpha.test\nbeta.test', 'missing', 0)).toBeNull();
  });
});
