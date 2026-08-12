#!/usr/bin/env bash
# Fetch a prebuilt catalog tarball from GitHub Releases and install it to
# web/public/catalog/. This lets contributors skip the 15-45 minute
# `cargo run -p elfo-catalog --release -- gen ...` regeneration.
#
# Usage:
#   scripts/fetch-catalog.sh              # latest release
#   scripts/fetch-catalog.sh catalog-abc1234  # a specific release tag
set -euo pipefail

tag="${1:-}"

if [[ -n "$tag" ]]; then
  url="https://github.com/translunar/frozen/releases/download/${tag}/catalog.tar.gz"
  tag_label="$tag"
else
  url="https://github.com/translunar/frozen/releases/latest/download/catalog.tar.gz"
  tag_label="latest"
fi

# Resolve the repo root regardless of cwd: prefer `git rev-parse`, fall back
# to the script's own location.
if repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
  :
else
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  repo_root="$(cd "${script_dir}/.." && pwd)"
fi

catalog_dir="${repo_root}/web/public/catalog"

# Safety: never write through or replace a symlink. Developers commonly
# symlink web/public/catalog between worktrees to share one generated
# catalog; blowing that away would silently corrupt the other worktree.
if [[ -L "$catalog_dir" ]]; then
  echo "error: ${catalog_dir} is a symlink (likely pointing at another worktree's catalog)." >&2
  echo "Refusing to overwrite it. Remove the symlink yourself first if you really want to" >&2
  echo "replace it with a fetched catalog, e.g.:" >&2
  echo "  rm \"${catalog_dir}\" && $(basename "$0")${tag:+ $tag}" >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

archive="${tmp_dir}/catalog.tar.gz"
extract_dir="${tmp_dir}/extracted"
mkdir -p "$extract_dir"

echo "Downloading catalog (${tag_label}) from ${url} ..."
if ! curl -fL --progress-bar -o "$archive" "$url"; then
  echo "error: failed to download ${url}" >&2
  echo "hint: the release may not exist yet. You can regenerate the catalog from source instead:" >&2
  echo "  cargo run -p elfo-catalog --release -- gen --config catalog.toml --out web/public/catalog" >&2
  exit 1
fi

tar -xzf "$archive" -C "$extract_dir"

if [[ ! -f "${extract_dir}/catalog.json" ]]; then
  echo "error: extracted archive does not contain catalog.json at its top level" >&2
  echo "hint: the release asset may be malformed. You can regenerate the catalog from source instead:" >&2
  echo "  cargo run -p elfo-catalog --release -- gen --config catalog.toml --out web/public/catalog" >&2
  exit 1
fi

# Atomic-ish install: move the old tree aside, move the new tree in, then
# delete the old one. If anything goes wrong between the two moves, the
# .old directory is left behind rather than data being lost.
mkdir -p "${repo_root}/web/public"
old_dir="${repo_root}/web/public/catalog.old.$$"
if [[ -e "$catalog_dir" ]]; then
  mv "$catalog_dir" "$old_dir"
fi
mv "$extract_dir" "$catalog_dir"
if [[ -e "$old_dir" ]]; then
  rm -rf "$old_dir"
fi

size="$(du -sh "$catalog_dir" | cut -f1)"
echo "Installed catalog (${tag_label}) to ${catalog_dir} (${size})"
