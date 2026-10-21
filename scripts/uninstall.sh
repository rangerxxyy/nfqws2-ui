#!/bin/sh
set -eu

ROOT=/opt/share/nfqws2-ui
ETC=/opt/etc/nfqws2-ui
DATA=/opt/var/lib/nfqws2-ui
PURGE=0

case "${1:-}" in
	'') ;;
	--purge) PURGE=1 ;;
	*) echo "Usage: $0 [--purge]" >&2; exit 2 ;;
esac
[ "$(id -u)" = 0 ] || { echo 'Run as root' >&2; exit 1; }

if [ -x /opt/etc/init.d/S99nfqws2-ui ]; then
	/opt/etc/init.d/S99nfqws2-ui stop || { echo 'Could not stop nfqws2-ui; refusing removal' >&2; exit 1; }
fi
rm -f /opt/etc/init.d/S99nfqws2-ui /opt/var/run/nfqws2-ui.port
rm -rf "$ROOT" /opt/var/run/nfqws2-ui
rm -f "$ETC/password.hash"
rmdir "$ETC" 2>/dev/null || true
if [ "$PURGE" = 1 ]; then
	rm -rf "$ETC" "$DATA"
	 echo 'nfqws2-ui removed, including panel backups.'
else
	echo 'nfqws2-ui program removed; backups remain in /opt/var/lib/nfqws2-ui.'
fi
echo 'nfqws2 and its configuration were not changed.'
