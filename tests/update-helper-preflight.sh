#!/bin/sh
set -eu
ID=00112233445566778899aabbccddeeff
grep -Fq 'CORE_INIT=/opt/etc/init.d/S51nfqws2' scripts/update-release.sh || { echo 'panel updater does not target the nfqws2 service' >&2; exit 1; }
grep -Fq "\"\$CORE_INIT\" restart" scripts/update-release.sh || { echo 'panel updater does not restart nfqws2' >&2; exit 1; }
grep -Fq "\"\$CORE_INIT\" status" scripts/update-release.sh || { echo 'panel updater does not verify nfqws2 after restart' >&2; exit 1; }
grep -Fq "\"\$STAGE/scripts/nfqws2-ui-ctl\" prune-releases \"\$ROOT/releases\" \"\$STAGE\"" scripts/update-release.sh || { echo 'panel updater does not prune old releases after success' >&2; exit 1; }
for script in scripts/install.sh scripts/update-release.sh scripts/update-core-release.sh; do
	grep -Fq 'if [ -x /opt/bin/wget ]; then WGET=/opt/bin/wget' "$script" || { echo "$script does not prefer Entware wget" >&2; exit 1; }
done
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
root="$tmp/app"
sed "s#^ROOT=/opt/share/nfqws2-ui#ROOT=$root#" scripts/nfqws2-ui-ctl > "$tmp/nfqws2-ui-ctl"
chmod +x "$tmp/nfqws2-ui-ctl"
releases="$root/releases"
mkdir -p "$releases/current" "$releases/previous" "$releases/older"
sh "$tmp/nfqws2-ui-ctl" prune-releases "$releases" "$releases/current"
[ -d "$releases/current" ] || { echo 'current release was removed' >&2; exit 1; }
if [ -e "$releases/previous" ] || [ -e "$releases/older" ]; then
	echo 'old releases were not removed' >&2
	exit 1
fi
mkdir -p "$tmp/outside"
if sh "$tmp/nfqws2-ui-ctl" prune-releases "$releases" "$tmp/outside" >/dev/null 2>&1; then
	echo 'pruner accepted a current release outside the releases directory' >&2
	exit 1
fi
[ -d "$tmp/outside" ] || { echo 'pruner removed a path outside releases' >&2; exit 1; }
if sh "$tmp/nfqws2-ui-ctl" prune-releases "$tmp/outside" "$tmp/outside" >/dev/null 2>&1; then
	echo 'pruner accepted storage outside the application root' >&2
	exit 1
fi
if sh scripts/update-release.sh latest "$ID" >/dev/null 2>&1; then
	echo 'panel updater accepted an invalid release tag' >&2
	exit 1
fi
if sh scripts/update-core-release.sh latest nfqws2-keenetic_1.2.3_aarch64-3.10.ipk 0000000000000000000000000000000000000000000000000000000000000000 v1.2.2 nfqws2-keenetic_1.2.2_aarch64-3.10.ipk 0000000000000000000000000000000000000000000000000000000000000000 "$ID" >/dev/null 2>&1; then
	echo 'core updater accepted an invalid release tag' >&2
	exit 1
fi
if sh scripts/update-core-release.sh v1.2.3 ../package.ipk 0000000000000000000000000000000000000000000000000000000000000000 v1.2.2 nfqws2-keenetic_1.2.2_aarch64-3.10.ipk 0000000000000000000000000000000000000000000000000000000000000000 "$ID" >/dev/null 2>&1; then
	echo 'core updater accepted a path traversal package name' >&2
	exit 1
fi
echo 'Updater preflight checks passed.'
