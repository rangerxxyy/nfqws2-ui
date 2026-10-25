export type ReleaseInline =
  | { type: 'text'; text: string }
  | { type: 'code'; text: string }
  | { type: 'strong'; children: ReleaseInline[] }
  | { type: 'emphasis'; children: ReleaseInline[] }
  | { type: 'strike'; children: ReleaseInline[] }
  | { type: 'link'; href: string; children: ReleaseInline[] }
  | { type: 'break' };

export type ReleaseBlock =
  | { type: 'paragraph'; children: ReleaseInline[] }
  | { type: 'quote'; children: ReleaseInline[] }
  | { type: 'heading'; level: number; children: ReleaseInline[] }
  | { type: 'list'; ordered: boolean; items: ReleaseInline[][] }
  | { type: 'table'; headers: ReleaseInline[][]; alignments: Array<'left' | 'center' | 'right' | null>; rows: ReleaseInline[][][] }
  | { type: 'code-block'; text: string }
  | { type: 'rule' };

const MAX_RELEASE_MARKDOWN_LENGTH = 16_000;
const INLINE_MARKDOWN = /(`[^`\n]+`|\[[^\]\n]+\]\([^\s)]+(?:\s+"[^"]*")?\)|\*\*[^*\n]+\*\*|__[^_\n]+__|~~[^~\n]+~~|\*[^*\n]+\*|_[^_\n]+_)/g;
const LIST_LINE = /^\s*(?:([-+*])\s+|(\d+)[.)]\s+)(.*)$/;
const FENCE_LINE = /^\s*(```+|~~~+)(.*)$/;

function splitTableRow(line: string): string[] {
  const source = line.trim().replace(/^\|/, '').replace(/\|$/, '');
  const cells: string[] = [];
  let cell = '';
  let escaped = false;
  for (const char of source) {
    if (escaped) { cell += char; escaped = false; }
    else if (char === '\\') { cell += char; escaped = true; }
    else if (char === '|') { cells.push(cell.trim().replace(/\\\|/g, '|')); cell = ''; }
    else cell += char;
  }
  cells.push(cell.trim().replace(/\\\|/g, '|'));
  return cells;
}

function tableAlignments(line: string): Array<'left' | 'center' | 'right' | null> | null {
  const cells = splitTableRow(line);
  if (!cells.length || cells.some((cell) => !/^:?-+:?$/.test(cell))) return null;
  return cells.map((cell) => cell.startsWith(':') && cell.endsWith(':') ? 'center' : cell.endsWith(':') ? 'right' : cell.startsWith(':') ? 'left' : null);
}

function isTableStart(lines: string[], index: number): boolean {
  const current = lines[index] ?? '';
  const next = lines[index + 1] ?? '';
  return current.includes('|') && tableAlignments(next) !== null && splitTableRow(current).length === splitTableRow(next).length;
}

function safeHref(href: string): string | null {
  const value = href.trim().replace(/&amp;/g, '&');
  if (!value || /[\u0000-\u0020]/.test(value)) return null;
  try {
    const url = new URL(value, 'https://github.com');
    return ['https:', 'http:', 'mailto:'].includes(url.protocol) ? value : null;
  } catch {
    return null;
  }
}

function textToken(text: string): ReleaseInline {
  return { type: 'text', text };
}

function parseInline(source: string, depth = 0): ReleaseInline[] {
  if (depth > 6) return [textToken(source)];
  const tokens: ReleaseInline[] = [];
  INLINE_MARKDOWN.lastIndex = 0;
  let offset = 0;
  for (const match of source.matchAll(INLINE_MARKDOWN)) {
    const raw = match[0];
    const start = match.index ?? 0;
    if (start > offset) tokens.push(textToken(source.slice(offset, start)));
    if (raw.startsWith('`')) {
      tokens.push({ type: 'code', text: raw.slice(1, -1) });
    } else if (raw.startsWith('[')) {
      const splitAt = raw.indexOf('](');
      const label = raw.slice(1, splitAt);
      const hrefStart = splitAt + 2;
      const href = raw.slice(hrefStart, raw.indexOf(')', hrefStart)).replace(/^<|>$/g, '');
      const safe = safeHref(href);
      if (safe) tokens.push({ type: 'link', href: safe, children: parseInline(label, depth + 1) });
      else tokens.push(textToken(label));
    } else {
      const marker = raw.startsWith('**') || raw.startsWith('__') ? 2 : 1;
      const type = raw.startsWith('~~') ? 'strike' : marker === 2 ? 'strong' : 'emphasis';
      tokens.push({ type, children: parseInline(raw.slice(marker, -marker), depth + 1) });
    }
    offset = start + raw.length;
  }
  if (offset < source.length) tokens.push(textToken(source.slice(offset)));
  return tokens;
}

function beginsBlock(line: string): boolean {
  return /^\s*(?:#{1,6}\s+|>|(?:[-+*]\s+|\d+[.)]\s+)|(?:```|~~~)|(?:\s*(?:-{3,}|_{3,}|\*{3,})\s*$))/.test(line);
}

export function parseReleaseMarkdown(markdown: string): ReleaseBlock[] {
  const lines = markdown.slice(0, MAX_RELEASE_MARKDOWN_LENGTH).replace(/\r/g, '').split('\n');
  const blocks: ReleaseBlock[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i] ?? '';
    if (!line.trim()) { i += 1; continue; }
    const fence = line.match(FENCE_LINE);
    if (fence) {
      const marker = fence[1]?.[0] ?? '`';
      i += 1;
      const code: string[] = [];
      while (i < lines.length && !new RegExp(`^\\s*${marker}{3,}\\s*$`).test(lines[i] ?? '')) code.push(lines[i++] ?? '');
      if (i < lines.length) i += 1;
      blocks.push({ type: 'code-block', text: code.join('\n') });
      continue;
    }
    if (isTableStart(lines, i)) {
      const headers = splitTableRow(lines[i] ?? '').map(parseInline);
      const alignments = tableAlignments(lines[i + 1] ?? '') ?? [];
      i += 2;
      const rows: ReleaseInline[][][] = [];
      while (i < lines.length && (lines[i] ?? '').includes('|') && (lines[i] ?? '').trim()) {
        const cells = splitTableRow(lines[i] ?? '');
        rows.push(headers.map((_, index) => parseInline(cells[index] ?? '')));
        i += 1;
      }
      blocks.push({ type: 'table', headers, alignments, rows });
      continue;
    }
    const heading = line.match(/^\s*(#{1,6})\s+(.+?)\s*#*\s*$/);
    if (heading) { blocks.push({ type: 'heading', level: heading[1]?.length ?? 2, children: parseInline(heading[2] ?? '') }); i += 1; continue; }
    if (/^\s*(?:-{3,}|_{3,}|\*{3,})\s*$/.test(line)) { blocks.push({ type: 'rule' }); i += 1; continue; }
    if (/^\s*>/.test(line)) {
      const quote: string[] = [];
      while (i < lines.length && /^\s*>/.test(lines[i] ?? '')) quote.push((lines[i++] ?? '').replace(/^\s*>\s?/, ''));
      blocks.push({ type: 'quote', children: parseInline(quote.join(' ')) });
      continue;
    }
    const firstList = line.match(LIST_LINE);
    if (firstList) {
      const ordered = firstList[2] !== undefined;
      const items: ReleaseInline[][] = [];
      while (i < lines.length) {
        const item = (lines[i] ?? '').match(LIST_LINE);
        if (!item || (item[2] !== undefined) !== ordered) break;
        items.push(parseInline(item[3] ?? ''));
        i += 1;
      }
      blocks.push({ type: 'list', ordered, items });
      continue;
    }
    const paragraph: string[] = [];
    while (i < lines.length && (lines[i] ?? '').trim() && (paragraph.length === 0 || (!beginsBlock(lines[i] ?? '') && !isTableStart(lines, i)))) paragraph.push(lines[i++] ?? '');
    const joined: ReleaseInline[] = [];
    paragraph.forEach((part, index) => {
      const hardBreak = /(?: {2,}|\\)$/.test(part);
      joined.push(...parseInline(part.replace(/(?: {2,}|\\)$/, '')));
      if (hardBreak && index < paragraph.length - 1) joined.push({ type: 'break' });
      else if (index < paragraph.length - 1) joined.push(textToken(' '));
    });
    blocks.push({ type: 'paragraph', children: joined });
  }
  return blocks;
}

function renderInlineToken(token: ReleaseInline): Node {
  if (token.type === 'text' || token.type === 'code') {
    const node = document.createElement(token.type === 'code' ? 'code' : 'span');
    node.textContent = token.text;
    return node;
  }
  if (token.type === 'break') return document.createElement('br');
  const tag = token.type === 'strong' ? 'strong' : token.type === 'emphasis' ? 'em' : token.type === 'strike' ? 'del' : 'a';
  const node = document.createElement(tag);
  if (token.type === 'link') {
    node.setAttribute('href', token.href);
    node.setAttribute('target', '_blank');
    node.setAttribute('rel', 'noopener noreferrer');
  }
  for (const child of token.children) node.append(renderInlineToken(child));
  return node;
}

export function renderReleaseMarkdown(markdown: string): HTMLDivElement {
  const container = document.createElement('div');
  container.className = 'release-markdown';
  for (const block of parseReleaseMarkdown(markdown)) {
    let node: HTMLElement;
    if (block.type === 'heading') node = document.createElement(`h${Math.min(6, Math.max(1, block.level))}`);
    else if (block.type === 'paragraph') node = document.createElement('p');
    else if (block.type === 'quote') node = document.createElement('blockquote');
    else if (block.type === 'rule') { container.append(document.createElement('hr')); continue; }
    else if (block.type === 'code-block') {
      node = document.createElement('pre');
      const code = document.createElement('code'); code.textContent = block.text; node.append(code);
      container.append(node); continue;
    } else if (block.type === 'table') {
      const wrapper = document.createElement('div'); wrapper.className = 'release-table-scroll';
      const table = document.createElement('table'); table.className = 'release-table';
      const head = document.createElement('thead'); const headerRow = document.createElement('tr');
      block.headers.forEach((tokens, index) => {
        const cell = document.createElement('th');
        cell.scope = 'col';
        const align = block.alignments[index]; if (align) cell.style.textAlign = align;
        for (const token of tokens) cell.append(renderInlineToken(token));
        headerRow.append(cell);
      });
      head.append(headerRow); table.append(head);
      const body = document.createElement('tbody');
      for (const row of block.rows) {
        const tr = document.createElement('tr');
        row.forEach((tokens, index) => {
          const cell = document.createElement('td');
          const align = block.alignments[index]; if (align) cell.style.textAlign = align;
          for (const token of tokens) cell.append(renderInlineToken(token));
          tr.append(cell);
        });
        body.append(tr);
      }
      table.append(body); wrapper.append(table); container.append(wrapper); continue;
    } else {
      node = document.createElement(block.ordered ? 'ol' : 'ul');
      for (const item of block.items) {
        const listItem = document.createElement('li');
        for (const token of item) listItem.append(renderInlineToken(token));
        node.append(listItem);
      }
      container.append(node); continue;
    }
    for (const token of block.children) node.append(renderInlineToken(token));
    container.append(node);
  }
  return container;
}
