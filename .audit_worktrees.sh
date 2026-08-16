#!/usr/bin/env bash
# Worktree audit for Voxelforge - read-only, no commit/push/stash/prune

REPO_ROOT="E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge"
cd "$REPO_ROOT"

echo "# Voxelforge worktree audit"
echo ""
echo "| Worktree | Branch / HEAD | Ahead of poppy/native-only | Unpushed commits | Pending files | Work-like untracked/notes |"
echo "|----------|---------------|----------------------------|------------------|---------------|---------------------------|"

worktrees=()
while IFS= read -r line; do
  if [[ "$line" == worktree* ]]; then
    wt="${line#worktree }"
    worktrees+=("$wt")
  fi
done < <(git worktree list --porcelain)

for wt in "${worktrees[@]}"; do
  cd "$wt"

  head_info=$(git rev-parse --abbrev-ref HEAD)
  if [[ "$head_info" == "HEAD" ]]; then
    head_info="detached $(git rev-parse --short HEAD)"
  fi

  merge_base=$(git merge-base HEAD poppy/native-only 2>/dev/null || true)
  if [[ -n "$merge_base" ]]; then
    ahead_of_poppy=$(git rev-list --count "$merge_base..HEAD" 2>/dev/null || echo "?")
  else
    ahead_of_poppy="?"
  fi

  upstream=$(git rev-parse --abbrev-ref HEAD@{u} 2>/dev/null || true)
  if [[ -n "$upstream" ]]; then
    # commits on local branch not yet in upstream
    unpushed=$(git rev-list --count "$upstream..HEAD" 2>/dev/null || echo "?")
  else
    unpushed="N/A"
  fi

  pending_count=$(git status --short 2>/dev/null | wc -l | tr -d ' ')

  work_like=""
  if [[ "$pending_count" -gt 0 ]]; then
    pending_raw=$(git status --short 2>/dev/null)
    binaries=$(echo "$pending_raw" | grep -iE '\.(png|ktx2|jpg|jpeg|gif|webp|mp3|wav|ogg|glb|gltf|fbx|obj|exe|dll|bin)$' | head -5 | sed 's/^...//' | tr '\n' ' ' || true)
    src=$(echo "$pending_raw" | grep -iE '\.(rs|toml|md|json|cmd|ps1|sh|py|js|ts|tsx|css|html)$' | head -5 | sed 's/^...//' | tr '\n' ' ' || true)
    if [[ -n "$binaries" ]]; then
      work_like="BIN: $binaries"
    fi
    if [[ -n "$src" ]]; then
      work_like="$work_like SRC: $src"
    fi
    work_like=$(echo "$work_like" | sed 's/  */ /g' | cut -c1-120)
  fi

  wt_short=$(echo "$wt" | sed "s|E:/Projects/bagidea-ai-agents-office/workspace/projects/||")

  printf "| %s | %s | %s | %s | %s | %s |\n" "$wt_short" "$head_info" "$ahead_of_poppy" "$unpushed" "$pending_count" "$work_like"
done
