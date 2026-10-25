#!/bin/sh
set -eu

INSTALLER=${1:-scripts/install.sh}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

if ! sed -n '/^download_and_verify_release_archive() {/,/^}/p' "$INSTALLER" > "$TMP/download-function.sh"; then
	echo 'could not extract release download function' >&2
	exit 1
fi
grep -q '^download_and_verify_release_archive() {' "$TMP/download-function.sh" || {
	echo 'release download function is missing' >&2
	exit 1
}

cat > "$TMP/wget" <<'EOF'
#!/bin/sh
set -eu
[ "$1" = -q ]
[ "$2" = -O ]
output=$3
url=$4
case "$url" in *\?nfqws2-ui-cache=*) ;; *) exit 21 ;; esac
cache=${url##*nfqws2-ui-cache=}
case "$url" in
	*.sha256?*)
		[ "$cache" = "$(cat "$CURRENT_CACHE_FILE")" ] || exit 22
		printf '%s  nfqws2-ui-test.tar.gz\n' "$EXPECTED_HASH" > "$output"
		;;
	*)
		ARCHIVE_CALLS=$(cat "$CALLS_FILE" 2>/dev/null || printf '0')
		ARCHIVE_CALLS=$((ARCHIVE_CALLS + 1))
		printf '%s' "$ARCHIVE_CALLS" > "$CALLS_FILE"
		PREVIOUS_CACHE=$(cat "$LAST_CACHE_FILE" 2>/dev/null || true)
		printf '%s' "$cache" > "$CURRENT_CACHE_FILE"
		printf '%s' "$cache" > "$LAST_CACHE_FILE"
		if [ "$RETRY_MODE" = mismatch ] || { [ "$ARCHIVE_CALLS" -gt 1 ] && [ "$cache" = "$PREVIOUS_CACHE" ]; }; then
			printf 'stale archive bytes\n' > "$output"
		elif [ "$ARCHIVE_CALLS" -eq 1 ]; then
			printf 'stale archive bytes\n' > "$output"
		else
			printf 'verified archive bytes\n' > "$output"
		fi
		;;
esac
EOF
chmod +x "$TMP/wget"

GOOD_HASH=$(printf 'verified archive bytes\n' | sha256sum | awk '{print $1}')
WGET="$TMP/wget"
VERSION=0.9.5
EXPECTED_HASH=$GOOD_HASH
RETRY_MODE=retry
CALLS_FILE="$TMP/archive-calls"
CURRENT_CACHE_FILE="$TMP/current-cache"
LAST_CACHE_FILE="$TMP/last-cache"
export EXPECTED_HASH RETRY_MODE CALLS_FILE CURRENT_CACHE_FILE LAST_CACHE_FILE
. "$TMP/download-function.sh"

if ! download_and_verify_release_archive \
	'https://example.invalid/releases/download/v0.9.5' \
	'nfqws2-ui-test.tar.gz' \
	'nfqws2-ui-test.tar.gz.sha256' \
	"$TMP"; then
	echo 'release download was not retried after a stale cached archive' >&2
	exit 1
fi
[ "$(cat "$CALLS_FILE")" -eq 2 ] || { echo "expected 2 archive attempts, got $(cat "$CALLS_FILE")" >&2; exit 1; }
printf 'verified archive bytes\n' | cmp - "$TMP/nfqws2-ui-test.tar.gz"

printf '0' > "$CALLS_FILE"
rm -f "$LAST_CACHE_FILE"
RETRY_MODE=mismatch
if download_and_verify_release_archive \
	'https://example.invalid/releases/download/v0.9.5' \
	'nfqws2-ui-test.tar.gz' \
	'nfqws2-ui-test.tar.gz.sha256' \
	"$TMP"; then
	echo 'release download accepted a persistently mismatched archive' >&2
	exit 1
fi
[ "$(cat "$CALLS_FILE")" -eq 3 ] || { echo "expected 3 bounded archive attempts, got $(cat "$CALLS_FILE")" >&2; exit 1; }

echo 'installer cache retry behavior passed'
