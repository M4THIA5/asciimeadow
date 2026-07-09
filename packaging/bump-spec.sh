#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: bump-spec.sh <version> [spec-path]}"
spec="${2:-packaging/asciimeadow.spec}"
author="Mathias Collas <mathias.collas@gmail.com>"
date_str="$(LC_ALL=C date '+%a %b %d %Y')"

if [[ ! "$version" =~ ^[0-9A-Za-z.+\-]+$ ]]; then
  echo "version invalide: $version" >&2
  exit 1
fi

sed -i "s/^Version:.*/Version:        ${version}/" "$spec"

sed -i "/^%changelog$/a * ${date_str} ${author} - ${version}-1\n- Release ${version}\n" "$spec"

if ! grep -q "^- Release ${version}$" "$spec"; then
  echo "insertion changelog échouée: ancre %changelog introuvable" >&2
  exit 1
fi
