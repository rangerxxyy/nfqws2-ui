export const MAX_FOREIGN_CONFIG_BYTES = 65_536;

export function decodeForeignConfigText(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let decoder: TextDecoder;
  let offset = 0;

  if (bytes[0] === 0xff && bytes[1] === 0xfe) {
    decoder = new TextDecoder('utf-16le', { fatal: true });
    offset = 2;
  } else if (bytes[0] === 0xfe && bytes[1] === 0xff) {
    decoder = new TextDecoder('utf-16be', { fatal: true });
    offset = 2;
  } else {
    decoder = new TextDecoder('utf-8', { fatal: true });
    if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) offset = 3;
  }

  let content: string;
  try {
    content = decoder.decode(bytes.subarray(offset));
  } catch {
    const hasUtf8Bom = bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf;
    if (decoder.encoding !== 'utf-8' || hasUtf8Bom) {
      throw new Error('Кодировка файла не поддерживается. Сохраните конфиг в UTF-8, UTF-16 с BOM или Windows-1251.');
    }
    try {
      content = new TextDecoder('windows-1251', { fatal: true }).decode(bytes);
    } catch {
      throw new Error('Кодировка файла не поддерживается. Сохраните конфиг в UTF-8, UTF-16 с BOM или Windows-1251.');
    }
  }

  if (/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(content)) {
    throw new Error('Похоже, файл не является обычным текстовым конфигом.');
  }

  if (new TextEncoder().encode(content).byteLength > MAX_FOREIGN_CONFIG_BYTES) {
    throw new Error('Размер текста после декодирования не должен превышать 64 КиБ.');
  }
  return content;
}
