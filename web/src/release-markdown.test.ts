import { describe, expect, it } from 'vitest';
import { parseReleaseMarkdown } from './release-markdown';

describe('release Markdown', () => {
  it('preserves GitHub release headings, emphasis, code, links, lists, and quotes', () => {
    const blocks = parseReleaseMarkdown([
      '## v0.6.0',
      '',
      'Исправлено **важное**: [скачать](https://github.com/rangerxxyy/nfqws2-ui/releases) и `nfqws2`.',
      '',
      '- Первый пункт',
      '- Второй пункт',
      '',
      '> Белый акцентный текст',
    ].join('\n'));
    expect(blocks.map((block) => block.type)).toEqual(['heading', 'paragraph', 'list', 'quote']);
    if (blocks[1]?.type !== 'paragraph' || blocks[2]?.type !== 'list' || blocks[3]?.type !== 'quote') throw new Error('Unexpected parsed release content');
    expect(blocks[1].children.map((token) => token.type)).toEqual(['text', 'strong', 'text', 'link', 'text', 'code', 'text']);
    expect(blocks[2].items.map((item) => item[0]?.type === 'text' ? item[0].text : '')).toEqual(['Первый пункт', 'Второй пункт']);
    expect(blocks[3].children[0]?.type).toBe('text');
  });

  it('does not create links for unsafe schemes or interpret raw HTML', () => {
    const blocks = parseReleaseMarkdown('[опасно](javascript:alert(1)) <script>alert(1)</script>');
    expect(blocks[0]?.type).toBe('paragraph');
    if (blocks[0]?.type !== 'paragraph') throw new Error('Expected paragraph');
    expect(blocks[0].children.some((token) => token.type === 'link')).toBe(false);
    expect(blocks[0].children.map((token) => token.type === 'text' || token.type === 'code' ? token.text : '').join('')).toContain('<script>');
  });
});
