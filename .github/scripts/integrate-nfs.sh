#!/usr/bin/env bash
# Integrate maintained fix commits into an upstream release without changing the control branch.
set -euo pipefail

upstream_tag="${1:?Usage: integrate-nfs.sh <upstream-tag> <source-directory>}"
source_dir="${2:?Provide an empty source directory}"
control_dir="$(git rev-parse --show-toplevel)"
mapfile -t patches < "$control_dir/.github/nfs-patches"

git fetch --depth=1 https://github.com/openai/codex.git "refs/tags/$upstream_tag:refs/tags/$upstream_tag"
git worktree add --detach "$source_dir" "$upstream_tag"
git -C "$source_dir" config user.name 'github-actions[bot]'
git -C "$source_dir" config user.email '41898282+github-actions[bot]@users.noreply.github.com'

for patch in "${patches[@]}"; do
  [[ -z "$patch" || "$patch" == \#* ]] && continue
  # Fetch the parent too: cherry-pick needs the original commit's diff.
  git fetch --depth=2 origin "$patch"
  if ! git -C "$source_dir" cherry-pick -x "$patch"; then
    echo "NFS fix $patch no longer applies to $upstream_tag." >&2
    echo "Adapt the fix on a branch based on $upstream_tag, then update .github/nfs-patches." >&2
    git -C "$source_dir" cherry-pick --abort
    exit 1
  fi
done

integrated_commit="$(git -C "$source_dir" rev-parse HEAD)"
git worktree remove "$source_dir"
# Build from the integrated checkout so upstream package/build helpers match its version.
git switch --detach "$integrated_commit"
