import { describe, expect, it } from 'vitest';
import { appendListEntries, deduplicateListContent, isListTabSelected, listIdFromTabElementId, listTabIdsFromInventory, listRowsFromContent, removeListLines, removeListLinesByIndices, toggleListComments } from './list-editor';

describe('list editor helpers', () => {
  it('builds searchable rows without losing source line indexes or comment identity', () => {
    expect(listRowsFromContent('one.example\n# note\n\ntwo.example', 'TWO')).toEqual([
      { index: 3, text: 'two.example', comment: false },
    ]);
    expect(listRowsFromContent('one.example\n# note\n\ntwo.example')).toEqual([
      { index: 0, text: 'one.example', comment: false },
      { index: 1, text: '# note', comment: true },
      { index: 3, text: 'two.example', comment: false },
    ]);
  });

  it('removes selected source rows while retaining line endings and unselected comments', () => {
    expect(removeListLinesByIndices('one.example\r\n# keep\r\ntwo.example\r\n', [0, 2])).toEqual({
      content: '# keep\r\n',
      removed: 2,
    });
  });

  it('comments selected list lines and removes comment markers when all selected lines are commented', () => {
    expect(toggleListComments('one.example\ntwo.example\n', 0, 22)).toEqual({ content: '# one.example\n# two.example\n', selectionStart: 0, selectionEnd: 27 });
    expect(toggleListComments('  # one.example\n# two.example', 0, 30)).toEqual({ content: '  one.example\ntwo.example', selectionStart: 0, selectionEnd: 25 });
  });

  it('toggles only the current line when there is no selection and preserves blank lines', () => {
    expect(toggleListComments('one.example\n\ntwo.example', 13, 13)).toEqual({ content: 'one.example\n\n# two.example', selectionStart: 15, selectionEnd: 15 });
  });
  it('resolves each list tab from its element id so availability checks match backend ids', () => {
    expect(listIdFromTabElementId('listTab-user.list')).toBe('user.list');
    expect(listIdFromTabElementId('listTab-auto.list')).toBe('auto.list');
    expect(listIdFromTabElementId('listTab-exclude.list')).toBe('exclude.list');
    expect(listIdFromTabElementId('listTab-ipset.list')).toBe('ipset.list');
    expect(listIdFromTabElementId('listTab-ipset_exclude.list')).toBe('ipset_exclude.list');
    expect(listIdFromTabElementId('listTab-google.list')).toBe('google.list');
    expect(listIdFromTabElementId('listTab-../../nfqws2.conf')).toBeNull();
  });

  it('keeps safe custom list IDs from the backend inventory and filters invalid or duplicate IDs', () => {
    expect(listTabIdsFromInventory([
      { id: 'user.list' },
      { id: 'google.list' },
      { id: '../outside.list' },
      { id: 'google.list' },
      { id: 'nested/other.list' },
      { id: 'not-a-list.txt' },
    ])).toEqual(['user.list', 'google.list']);
  });

  it('selects the active list tab by its DOM id', () => {
    expect(isListTabSelected('listTab-user.list', 'user.list')).toBe(true);
    expect(isListTabSelected('listTab-auto.list', 'user.list')).toBe(false);
    expect(isListTabSelected('listTab-unknown.list', 'user.list')).toBe(false);
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

});
