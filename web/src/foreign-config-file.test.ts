import { describe, expect, it } from 'vitest';
import { isSupportedForeignConfigFileName } from './foreign-config-file';

describe('foreign config file names', () => {
  it.each(['nfqws2.conf', 'custom.txt', 'ROUTER.TXT', 'BACKUP.CONF'])(
    'accepts supported config filename %s',
    (name) => {
      expect(isSupportedForeignConfigFileName(name)).toBe(true);
    },
  );

  it.each(['config', 'config.conf.bak', 'script.sh', 'config.txt.exe', ''])(
    'rejects unsupported filename %s',
    (name) => {
      expect(isSupportedForeignConfigFileName(name)).toBe(false);
    },
  );
});
