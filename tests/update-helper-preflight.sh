#!/bin/sh
set -eu
ID=00112233445566778899aabbccddeeff
grep -Fq 'CORE_INIT=/opt/etc/init.d/S51nfqws2' scripts/update-release.sh || { echo 'panel updater does not target the nfqws2 service' >&2; exit 1; }
grep -Fq "\"\$CORE_INIT\" restart" scripts/update-release.sh || { echo 'panel updater does not restart nfqws2' >&2; exit 1; }
grep -Fq "\"\$CORE_INIT\" status" scripts/update-release.sh || { echo 'panel updater does not verify nfqws2 after restart' >&2; exit 1; }
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
