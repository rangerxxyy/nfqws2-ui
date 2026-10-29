export type LuaTokenType = 'plain' | 'comment' | 'string' | 'number' | 'keyword' | 'operator' | 'word';
export interface LuaToken { type: LuaTokenType; text: string }

const keywords = new Set(['and', 'break', 'do', 'else', 'elseif', 'end', 'false', 'for', 'function', 'if', 'in', 'local', 'nil', 'not', 'or', 'repeat', 'return', 'then', 'true', 'until', 'while', 'goto']);
const operators = ['...', '//', '==', '~=', '<=', '>=', '<<', '>>', '::', '..', '+', '-', '*', '/', '%', '^', '#', '=', '<', '>', '(', ')', '{', '}', '[', ']', ';', ':', ',', '.'];

function longBracket(source: string, start: number): { end: number } | null {
  const opening = source.slice(start).match(/^\[(=*)\[/);
  if (!opening) return null;
  const close = ']' + opening[1] + ']';
  const closeAt = source.indexOf(close, start + opening[0].length);
  return { end: closeAt < 0 ? source.length : closeAt + close.length };
}

export function tokenizeLuaScript(source: string): LuaToken[] {
  const tokens: LuaToken[] = [];
  const push = (type: LuaTokenType, text: string) => {
    if (!text) return;
    const previous = tokens.at(-1);
    if (previous?.type === type) previous.text += text;
    else tokens.push({ type, text });
  };
  let index = 0;
  while (index < source.length) {
    const char = source[index] ?? '';
    if (/\s/.test(char)) {
      const start = index++;
      while (index < source.length && /\s/.test(source[index] ?? '')) index += 1;
      push('plain', source.slice(start, index));
      continue;
    }
    if (source.startsWith('--', index)) {
      const block = longBracket(source, index + 2);
      if (block) { push('comment', source.slice(index, block.end)); index = block.end; continue; }
      const newline = source.indexOf('\n', index);
      const end = newline < 0 ? source.length : newline;
      push('comment', source.slice(index, end)); index = end; continue;
    }
    const bracket = longBracket(source, index);
    if (bracket) { push('string', source.slice(index, bracket.end)); index = bracket.end; continue; }
    if (char === '"' || char === "'") {
      const start = index++;
      let escaped = false;
      while (index < source.length) {
        const current = source[index++] ?? '';
        if (current === char && !escaped) break;
        if (current === '\\' && !escaped) escaped = true;
        else escaped = false;
      }
      push('string', source.slice(start, index));
      continue;
    }
    const identifier = source.slice(index).match(/^[A-Za-z_][A-Za-z0-9_]*/)?.[0];
    if (identifier) { push(keywords.has(identifier) ? 'keyword' : 'word', identifier); index += identifier.length; continue; }
    const number = source.slice(index).match(/^(?:0[xX][\da-fA-F]+|(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)/)?.[0];
    if (number) { push('number', number); index += number.length; continue; }
    const operator = operators.find((candidate) => source.startsWith(candidate, index));
    if (operator) { push('operator', operator); index += operator.length; continue; }
    push('plain', char); index += 1;
  }
  return tokens;
}
