import { describe, expect, it } from 'vitest';
import { tokenizeLuaScript } from './lua-syntax';

describe('Lua syntax tokens', () => {
  it('colors Lua keywords, strings, numbers, operators, and comments', () => {
    const source = 'local count = 2\nprint("hello") -- note';
    expect(tokenizeLuaScript(source)).toEqual([
      { type: 'keyword', text: 'local' }, { type: 'plain', text: ' ' },
      { type: 'word', text: 'count' }, { type: 'plain', text: ' ' },
      { type: 'operator', text: '=' }, { type: 'plain', text: ' ' },
      { type: 'number', text: '2' }, { type: 'plain', text: '\n' },
      { type: 'word', text: 'print' }, { type: 'operator', text: '(' },
      { type: 'string', text: '"hello"' }, { type: 'operator', text: ')' },
      { type: 'plain', text: ' ' }, { type: 'comment', text: '-- note' },
    ]);
  });

  it('preserves long strings, long comments, and markup as inert text', () => {
    const source = 'local blob = [=[<unsafe>]=] --[=[ explanation\ncontinued ]=]';
    const tokens = tokenizeLuaScript(source);
    expect(tokens.map((token) => token.text).join('')).toBe(source);
    expect(tokens).toContainEqual({ type: 'string', text: '[=[<unsafe>]=]' });
    expect(tokens).toContainEqual({ type: 'comment', text: '--[=[ explanation\ncontinued ]=]' });
  });
});
