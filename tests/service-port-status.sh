#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TMP=$(mktemp -d "${TMPDIR:-/tmp}/nfqws2-ui-service-port-status.XXXXXX")
RUN="$TMP/run"
PROC="$TMP/proc"
PORT_FILE="$TMP/nfqws2-ui.port"
STATUS="$TMP/S99nfqws2-ui"
PID=4242
PORT=43123

cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT HUP INT TERM

mkdir -p "$RUN" "$PROC/net" "$PROC/$PID"
printf '%s\n' "$PID" > "$RUN/pid"
printf '%s\n' "$ROOT/bin/nfqws2-ui" > "$PROC/$PID/cmdline"
printf '%s\n' "$PORT" > "$PORT_FILE"
printf 'sl local_address rem_address st\n' > "$PROC/net/tcp"
printf 'sl local_address rem_address st\n' > "$PROC/net/tcp6"

sed \
	-e "s|^APP=/opt/share/nfqws2-ui/current$|APP=$ROOT|" \
	-e "s|^RUN=/opt/var/run/nfqws2-ui$|RUN=$RUN|" \
	-e "s|^LOG=/opt/var/log/nfqws2-ui.log$|LOG=$TMP/nfqws2-ui.log|" \
	-e "s|^PORT_FILE=/opt/var/run/nfqws2-ui.port$|PORT_FILE=$PORT_FILE|" \
	-e "s|/proc/|$PROC/|g" \
	"$ROOT/scripts/S99nfqws2-ui" > "$STATUS"
chmod +x "$STATUS"

if OUTPUT=$("$STATUS" status 2>&1); then
	echo 'status reported a running panel without a listening TCP port' >&2
	exit 1
fi
printf '%s\n' "$OUTPUT" | grep -F "port $PORT is not listening" >/dev/null

PORT_HEX=$(printf '%04X' "$PORT")
printf '0: 00000000:%s 00000000:0000 0A\n' "$PORT_HEX" > "$PROC/net/tcp"
"$STATUS" status | grep -F "port $PORT is listening" >/dev/null

rm -f "$PORT_FILE"
if OUTPUT=$("$STATUS" status 2>&1); then
	echo 'status reported a running panel without a published port' >&2
	exit 1
fi
printf '%s\n' "$OUTPUT" | grep -F 'port is unknown' >/dev/null
