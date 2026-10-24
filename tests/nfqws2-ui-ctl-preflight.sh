#!/bin/sh
set -eu

SCRIPT=scripts/nfqws2-ui-ctl
HELP=$(sh "$SCRIPT" help)
printf '%s\n' "$HELP" | grep -Fq 'nfqws2-ui-ctl update --check'
printf '%s\n' "$HELP" | grep -Fq 'nfqws2-ui-ctl status'
if printf '%s\n' "$HELP" | grep -Fq 'nfqws-ui-setup'; then
	echo 'panel command help unexpectedly uses the zemidala executable name' >&2
	exit 1
fi
if sh "$SCRIPT" update v1.2.3.4 --check >/dev/null 2>&1; then
	echo 'panel command accepted an invalid semantic version' >&2
	exit 1
fi
if sh "$SCRIPT" unknown >/dev/null 2>&1; then
	echo 'panel command accepted an unknown action' >&2
	exit 1
fi
# shellcheck disable=SC2016
grep -Fq 'cp "$STAGE/scripts/nfqws2-ui-ctl" /opt/bin/nfqws2-ui-ctl' scripts/update-release.sh
grep -Fq 'rm -f /opt/bin/nfqws2-ui-ctl' scripts/uninstall.sh
echo 'nfqws2-ui-ctl preflight checks passed.'

