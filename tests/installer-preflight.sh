#!/bin/sh
set -eu

INSTALLER=${1:-scripts/install.sh}
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
cat > "$TMP/bin/opkg" <<'EOF'
#!/bin/sh
echo called >> "$OPKG_CALLED"
exit 1
EOF
chmod +x "$TMP/bin/"*
sed 's/@VERSION@/0.1.0/g' "$INSTALLER" > "$TMP/install.sh"

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
