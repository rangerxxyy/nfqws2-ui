#!/bin/sh
# Install a verified nfqws2-ui GitHub release and roll back if its health check fails.
set -eu
TAG=${1:-}
ID=${2:-}
printf '%s\n' "$TAG" | grep -Eq '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$' || { echo 'invalid release tag' >&2; exit 2; }
case "$ID" in *[!0-9a-fA-F]*|'') echo 'invalid operation id' >&2; exit 2 ;; esac
[ "${#ID}" -eq 32 ] || { echo 'invalid operation id' >&2; exit 2; }
ROOT=/opt/share/nfqws2-ui
DATA=/opt/var/lib/nfqws2-ui/update-operations
INIT=/opt/etc/init.d/S99nfqws2-ui
CORE_INIT=/opt/etc/init.d/S51nfqws2
PORT_FILE=/opt/var/run/nfqws2-ui.port
TMP="/tmp/nfqws2-ui-update-$ID"
STAGE="$ROOT/releases/$TAG-update-$ID"
CURRENT="$ROOT/current"
PREVIOUS=''
SWITCHED=0
COMMITTED=0
BASE="https://github.com/rangerxxyy/nfqws2-ui/releases/download/$TAG"
ASSET="nfqws2-ui-$TAG-aarch64-unknown-linux-musl.tar.gz"
set_state() {
	state=$1
	shift
	printf '%s\n%s\n' "$state" "$*" > "$DATA/$ID.state.tmp"
	mv -f "$DATA/$ID.state.tmp" "$DATA/$ID.state"
}
rollback() {
	[ "$SWITCHED" = 1 ] || return 0
	"$INIT" stop >/dev/null 2>&1 || true
	if [ -n "$PREVIOUS" ]; then
		ln -s "$PREVIOUS" "$ROOT/current.rollback.$ID"
		. "$STAGE/scripts/symlink.sh"
		replace_symlink "$ROOT/current.rollback.$ID" "$CURRENT" || return 1
		"$INIT" start >/dev/null 2>&1 || true
	else
		rm -f "$CURRENT" "$INIT"
	fi
	"$CORE_INIT" restart >/dev/null 2>&1 || true
	SWITCHED=0
}
# These callbacks are invoked by POSIX shell traps.
# shellcheck disable=SC2317
cleanup() {
	if [ "$COMMITTED" != 1 ]; then rollback || true; rm -rf "$STAGE"; fi
	rm -rf "$TMP"
}
# shellcheck disable=SC2317
on_abort() {
	if rollback; then set_state rolled_back 'Update interrupted; previous version restored.' || true
	else set_state failed 'Update interrupted and rollback failed.' || true; fi
	cleanup
	exit 1
}
trap cleanup EXIT
trap on_abort HUP INT TERM
[ "$(id -u)" = 0 ] || { set_state failed 'Root privileges are required.'; exit 1; }
command -v wget >/dev/null 2>&1 || { set_state failed 'wget is not installed.'; exit 1; }
command -v tar >/dev/null 2>&1 || { set_state failed 'tar is not installed.'; exit 1; }
command -v sha256sum >/dev/null 2>&1 || { set_state failed 'sha256sum is not installed.'; exit 1; }
mkdir -p "$ROOT/releases" "$DATA" "$TMP" || { set_state failed 'Could not create update directories.'; exit 1; }
chmod 700 "$DATA"
[ -x "$CORE_INIT" ] || { set_state failed 'nfqws2 service script is missing; panel update cancelled.'; exit 1; }
set_state downloading "Downloading release $TAG from GitHub."
wget -q -O "$TMP/$ASSET" "$BASE/$ASSET" || { set_state failed 'Could not download the release archive.'; exit 1; }
wget -q -O "$TMP/$ASSET.sha256" "$BASE/$ASSET.sha256" || { set_state failed 'Could not download the SHA-256 file.'; exit 1; }
set_state verifying 'Checking SHA-256 and archive contents.'
EXPECTED=$(awk 'NR == 1 { print $1 }' "$TMP/$ASSET.sha256")
case "$EXPECTED" in ''|*[!0-9a-fA-F]*) set_state failed 'Invalid SHA-256 file.'; exit 1 ;; esac
[ "${#EXPECTED}" -eq 64 ] || { set_state failed 'Invalid SHA-256 length.'; exit 1; }
ACTUAL=$(sha256sum "$TMP/$ASSET" | awk '{ print $1 }')
[ "$EXPECTED" = "$ACTUAL" ] || { set_state failed 'Archive SHA-256 mismatch.'; exit 1; }
tar -tzf "$TMP/$ASSET" > "$TMP/entries" || { set_state failed 'Invalid release archive.'; exit 1; }
while IFS= read -r entry; do
	case "$entry" in /*|../*|*/../*|*/..) set_state failed 'Archive contains an unsafe path.'; exit 1 ;; esac
done < "$TMP/entries"
if tar -tvzf "$TMP/$ASSET" | awk 'substr($1, 1, 1) != "-" && substr($1, 1, 1) != "d" { bad=1 } END { exit bad ? 0 : 1 }'; then
	set_state failed 'Archive contains links or special files.'
	exit 1
fi
mkdir -p "$STAGE"
tar -xzf "$TMP/$ASSET" -C "$STAGE" || { set_state failed 'Could not unpack the release archive.'; exit 1; }
if [ ! -x "$STAGE/bin/nfqws2-ui" ] || [ ! -d "$STAGE/public" ] || [ ! -x "$STAGE/scripts/S99nfqws2-ui" ] || [ ! -r "$STAGE/scripts/symlink.sh" ] || [ ! -x "$STAGE/scripts/update-release.sh" ] || [ ! -x "$STAGE/scripts/update-core-release.sh" ]; then
	set_state failed 'Release archive is missing required updater files.'
	exit 1
fi
. "$STAGE/scripts/symlink.sh"
if [ -L "$CURRENT" ]; then PREVIOUS=$(readlink "$CURRENT"); fi
set_state installing "Switching panel to $TAG."
"$INIT" stop >/dev/null 2>&1 || true
ln -s "$STAGE" "$ROOT/current.new.$ID"
SWITCHED=1
replace_symlink "$ROOT/current.new.$ID" "$CURRENT" || { rollback; set_state rolled_back 'Could not switch release; previous version restored.'; exit 1; }
set_state health_check 'Waiting for the panel health-check and nfqws2 service restart.'
"$INIT" start >/dev/null 2>&1 || { if rollback; then set_state rolled_back 'Service did not start; previous release restored.'; else set_state failed 'Service did not start and rollback failed.'; fi; exit 1; }
PORT=1002
[ ! -s "$PORT_FILE" ] || PORT=$(cat "$PORT_FILE")
case "$PORT" in ''|*[!0-9]*) PORT=1002 ;; esac
tries=0
while [ "$tries" -lt 20 ]; do
	if wget -q -O "$TMP/health" "http://127.0.0.1:$PORT/api/health" && grep -q '"ok":true' "$TMP/health"; then
		break
	fi
	tries=$((tries + 1))
	sleep 1
done
if [ "$tries" -eq 20 ]; then
	if rollback; then set_state rolled_back 'HTTP health-check failed; previous release restored.'
	else set_state failed 'HTTP health-check failed and rollback could not restore the previous release.'; fi
	exit 1
fi
if ! "$CORE_INIT" restart || ! "$CORE_INIT" status; then
	if rollback; then set_state rolled_back 'nfqws2 restart/status check failed; previous panel release restored.'
	else set_state failed 'nfqws2 restart/status check failed and panel rollback could not restore the previous release.'; fi
	exit 1
fi
SWITCHED=0
COMMITTED=1
set_state success "Panel $TAG passed HTTP health-check; nfqws2 restarted and status verified. Previous release retained."
exit 0
