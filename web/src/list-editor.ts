import type { NfqwsListId } from './api';

const STANDARD_LIST_IDS: readonly NfqwsListId[] = ['user.list', 'auto.list', 'exclude.list', 'ipset.list', 'ipset_exclude.list'];

export interface ListEditorRow {
  index: number;
  text: string;
  comment: boolean;
}

export function listRowsFromContent(content: string, query = ''): ListEditorRow[] {
  const needle = query.trim().toLocaleLowerCase('en-US');
  return content.split(/\r\n|\n/).flatMap((line, index) => {
    const text = line.trim();
    if (!text || (needle && !text.toLocaleLowerCase('en-US').includes(needle))) return [];
    return [{ index, text, comment: text.startsWith('#') }];
  });
}

export function removeListLinesByIndices(content: string, indices: readonly number[]): { content: string; removed: number } {
  const selected = new Set(indices.filter((index) => Number.isInteger(index) && index >= 0));
  if (!selected.size) return { content, removed: 0 };
  const lines = content.split(/\r\n|\n/);
  const remaining = lines.filter((_, index) => !selected.has(index));
  const removed = lines.length - remaining.length;
  return removed ? { content: remaining.join(content.includes('\r\n') ? '\r\n' : '\n'), removed } : { content, removed: 0 };
}

export function isSafeNfqwsListId(id: string): id is NfqwsListId {
  return id.length <= 128
    && /^[A-Za-z0-9._-]+\.list$/.test(id)
    && !id.startsWith('.')
    && !id.includes('..')
    && !STANDARD_LIST_IDS.some((standard) => standard.toLocaleLowerCase('en-US') === id.toLocaleLowerCase('en-US') && standard !== id);
}

export function listTabIdsFromInventory(files: readonly { id: string }[]): NfqwsListId[] {
  const seen = new Set<string>();
  return files.flatMap(({ id }) => {
    if (!isSafeNfqwsListId(id)) return [];
    const normalized = id.toLocaleLowerCase('en-US');
    if (seen.has(normalized)) return [];
    seen.add(normalized);
    return [id];
  });
}

export function listIdFromTabElementId(tabId: string): NfqwsListId | null {
  const id = tabId.startsWith('listTab-') ? tabId.slice('listTab-'.length) : '';
  return isSafeNfqwsListId(id) ? id : null;
}

export function isListTabSelected(tabElementId: string, activeId: NfqwsListId): boolean {
  return listIdFromTabElementId(tabElementId) === activeId;
}

export function appendListEntries(content: string, entries: string): string {
  const values = entries.split(/\r\n|\n/).map((entry) => entry.trim()).filter(Boolean);
  if (!values.length) return content;
  if (!content) return values.join('\n');
  const newline = content.includes('\r\n') ? '\r\n' : '\n';
  const endsWithNewline = content.endsWith('\n');
  const separator = endsWithNewline ? '' : newline;
  return `${content}${separator}${values.join(newline)}${endsWithNewline ? newline : ''}`;
}

export function removeListLines(content: string, selectionStart: number, selectionEnd: number): { content: string; removed: number } {
  if (!content) return { content, removed: 0 };
  const start = Math.max(0, Math.min(selectionStart, content.length));
  const end = Math.max(start, Math.min(selectionEnd, content.length));
  const firstLineStart = content.lastIndexOf('\n', Math.max(0, start - 1)) + 1;
  const finalSelectedOffset = end > start ? end - 1 : start;
  const nextNewline = content.indexOf('\n', finalSelectedOffset);
  const afterLastLine = nextNewline === -1 ? content.length : nextNewline + 1;
  const removed = content.slice(firstLineStart, afterLastLine).split(/\r\n|\n/).filter((line, index, lines) => !(index === lines.length - 1 && line === '')).length;
  return { content: content.slice(0, firstLineStart) + content.slice(afterLastLine), removed };
}

export function toggleListComments(content: string, selectionStart: number, selectionEnd: number): { content: string; selectionStart: number; selectionEnd: number } {
  const start = Math.max(0, Math.min(selectionStart, content.length));
  const end = Math.max(start, Math.min(selectionEnd, content.length));
  const from = content.lastIndexOf('\n', start - 1) + 1;
  const endAt = end > start && content[end - 1] === '\n' ? end - 1 : end;
  const newline = content.indexOf('\n', endAt);
  const to = newline < 0 ? content.length : newline;
  const lines = content.slice(from, to).split('\n');
  const nonempty = lines.filter((line) => line.trim());
  if (!nonempty.length) return { content, selectionStart: start, selectionEnd: end };
  const uncomment = nonempty.every((line) => /^\s*#/.test(line));
  const replacement = lines.map((line) => !line.trim() ? line : uncomment ? line.replace(/^(\s*)# ?/, '$1') : `# ${line}`).join('\n');
  const nextContent = content.slice(0, from) + replacement + content.slice(to);
  if (start === end && lines.length === 1) {
    const caret = Math.max(from, start + replacement.length - (to - from));
    return { content: nextContent, selectionStart: caret, selectionEnd: caret };
  }
  return { content: nextContent, selectionStart: from, selectionEnd: from + replacement.length };
}

export function findNextListMatch(content: string, query: string, caret: number): { start: number; end: number } | null {
  const needle = query.trim().toLocaleLowerCase('en-US');
  if (!needle) return null;
  const haystack = content.toLocaleLowerCase('en-US');
  const from = Math.max(0, Math.min(caret, content.length));
  const start = haystack.indexOf(needle, from);
  const wrappedStart = start === -1 ? haystack.indexOf(needle) : start;
  return wrappedStart === -1 ? null : { start: wrappedStart, end: wrappedStart + needle.length };
}

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
