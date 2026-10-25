import { describe, expect, it } from 'vitest';
import { decodeForeignConfigText } from './foreign-config-text';

function bytes(...values: number[]) {
  return new Uint8Array(values).buffer;
}

describe('foreign config text decoding', () => {
  it('decodes plain UTF-8 and removes a UTF-8 BOM', () => {
    expect(decodeForeignConfigText(bytes(0xef, 0xbb, 0xbf, 0x41, 0x0a))).toBe('A\n');
    expect(decodeForeignConfigText(bytes(0x41, 0x0a))).toBe('A\n');
  });

  it('decodes Windows UTF-16 little-endian and big-endian text files', () => {
    expect(decodeForeignConfigText(bytes(0xff, 0xfe, 0x41, 0x00, 0x0a, 0x00))).toBe('A\n');
    expect(decodeForeignConfigText(bytes(0xfe, 0xff, 0x00, 0x41, 0x00, 0x0a))).toBe('A\n');
  });

  it('falls back to Windows-1251 for legacy Russian text without a byte-order mark', () => {
    expect(decodeForeignConfigText(bytes(0xcf, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2))).toBe('Привет');
  });

  it('rejects binary control bytes and content exceeding the backend limit after decoding', () => {
    expect(() => decodeForeignConfigText(bytes(0xc3, 0x00))).toThrow('обычным текстовым конфигом');
    const tooLarge = new TextEncoder().encode('A'.repeat(65_537)).buffer;
    expect(() => decodeForeignConfigText(tooLarge)).toThrow('не должен превышать 64 КиБ');
  });
});
