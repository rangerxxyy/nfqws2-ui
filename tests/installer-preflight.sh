#!/bin/sh
set -eu

INSTALLER=${1:-scripts/install.sh}
if grep -Eq 'Create panel password|hash-password|/dev/tty' "$INSTALLER"; then
	echo 'installer still contains an interactive password setup' >&2
	exit 1
fi
grep -q 'успешно установлена' "$INSTALLER" || { echo 'installer success message is missing' >&2; exit 1; }
grep -q 'ip -o -4 addr show scope global' "$INSTALLER" || { echo 'installer LAN address detection is missing' >&2; exit 1; }
grep -Fq "\"\$STAGE/scripts/nfqws2-ui-ctl\" prune-releases \"\$ROOT/releases\" \"\$STAGE\"" "$INSTALLER" || { echo 'installer does not prune old releases after success' >&2; exit 1; }
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
mkdir "$TMP/bin"

cat > "$TMP/bin/id" <<'EOF'
#!/bin/sh
echo 0
EOF
cat > "$TMP/bin/wget" <<'EOF'
#!/bin/sh
exit 0
EOF
cat > "$TMP/bin/tar" <<'EOF'
#!/bin/sh
exit 0
EOF
cat > "$TMP/bin/date" <<'EOF'
#!/bin/sh
echo 20261021164000
EOF
cat > "$TMP/bin/rm" <<'EOF'
#!/bin/sh
exec /bin/rm "$@"
EOF
cat > "$TMP/bin/opkg" <<'EOF'
#!/bin/sh
echo called >> "$OPKG_CALLED"
exit 1
EOF
chmod +x "$TMP/bin/"*
sed "s/^VERSION='@VERSION@'$/VERSION='0.1.0'/" "$INSTALLER" > "$TMP/install.sh"

if OUTPUT=$(PATH="$TMP/bin" OPKG_CALLED="$TMP/opkg-called" /bin/sh "$TMP/install.sh" 2>&1); then
	echo 'installer unexpectedly passed its checksum dependency check' >&2
	exit 1
fi
case "$OUTPUT" in
	*'sha256sum is required'*) ;;
	*) echo "installer did not explain the missing checksum utility: $OUTPUT" >&2; exit 1 ;;
esac
[ ! -e "$TMP/opkg-called" ] || { echo 'installer invoked opkg during preflight' >&2; exit 1; }
echo 'installer checksum preflight passed without invoking opkg'

SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
sh "$SCRIPT_DIR/installer-download-retry.sh" "$INSTALLER"
