#!/usr/bin/env bash
# Test de packaging/bump-spec.sh sur une fixture .spec jetable.
set -euo pipefail

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
spec="$tmp/test.spec"

cat > "$spec" <<'EOF'
Name:           asciimeadow
Version:        0.1.0
Release:        1%{?dist}

%changelog
* Wed Jul 08 2026 Mathias Collas <mathias.collas@gmail.com> - 0.1.0-1
- Paquet initial
EOF

fail() { echo "FAIL: $1"; exit 1; }

# Test avec version invalide (caractères non-alphanumériques/semver)
version_before="$(grep '^Version:' "$spec")"
exit_code=0
bash packaging/bump-spec.sh 'a&b/c' "$spec" || exit_code=$?
version_after="$(grep '^Version:' "$spec")"

if [ "$exit_code" -eq 0 ]; then fail "version invalide: exit code devrait être non-zéro"; fi
if [ "$version_before" != "$version_after" ]; then fail "version invalide: spec ne devrait pas changer"; fi

# Test nominal (version valide)
bash packaging/bump-spec.sh 0.2.0 "$spec"

if ! grep -qE '^Version:        0\.2\.0$' "$spec"; then fail "Version non bumpée"; fi
if grep -qE '^Version:        0\.1\.0$' "$spec"; then fail "ancienne Version encore présente"; fi
if ! grep -q '^- Release 0.2.0$' "$spec"; then fail "ligne de release manquante"; fi

new_line="$(grep -n -- '- 0\.2\.0-1$' "$spec" | head -1 | cut -d: -f1)"
old_line="$(grep -n -- '- 0\.1\.0-1$' "$spec" | head -1 | cut -d: -f1)"
if [ -z "$new_line" ] || [ -z "$old_line" ]; then fail "entrées changelog introuvables"; fi
if [ "$new_line" -ge "$old_line" ]; then fail "nouvelle entrée pas en premier ($new_line >= $old_line)"; fi

echo "PASS"
