import { describe, expect, it } from 'vitest';
import { summarizeNfqwsConfig } from './config-facts';

describe('nfqws2 config facts shown in the panel', () => {
  it('reads the selected mode and multiline strategy variables without executing shell syntax', () => {
    const facts = summarizeNfqwsConfig(`MODE_LIST="--hostlist=/etc/nfqws2/lists/user.list"
MODE_AUTO="$MODE_LIST --hostlist-auto=/etc/nfqws2/lists/auto.list"
NFQWS_EXTRA_ARGS="$MODE_AUTO"
NFQWS_ARGS="--filter-tcp=443
--lua-desync=fake:strategy=1
--lua-desync=multisplit:strategy=1
# --lua-desync=commented-out"
NFQWS_ARGS_CUSTOM="--new=custom --filter-udp=443 --lua-desync=fake"
`);

    expect(facts.selectedMode).toBe('$MODE_AUTO');
    expect(facts.sections).toEqual([
      { variable: 'NFQWS_ARGS', strategies: 2, profiles: 0, filters: 1 },
      { variable: 'NFQWS_ARGS_CUSTOM', strategies: 1, profiles: 1, filters: 1 },
    ]);
  });

  it('does not turn unresolved shell expressions into a claimed mode or profile', () => {
    const facts = summarizeNfqwsConfig('NFQWS_EXTRA_ARGS="${UNKNOWN_MODE}"\nNFQWS_ARGS="$USER_ARGS"\n');
    expect(facts.selectedMode).toBe('${UNKNOWN_MODE}');
    expect(facts.sections).toEqual([]);
  });
});
