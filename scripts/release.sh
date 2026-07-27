#!/usr/bin/env bash

set -euo pipefail

fail() {
  printf 'release: %s\n' "$*" >&2
  exit 1
}

restore_uncommitted_version_change() {
  status=$?
  if test "$status" -ne 0 && test "$release_commit_created" = false; then
    git restore --staged --worktree --source=HEAD -- Cargo.toml Cargo.lock \
      >/dev/null 2>&1 || true
  fi
  exit "$status"
}

command -v cargo >/dev/null || fail "cargo is not available"
command -v jq >/dev/null || fail "jq is not available"
command -v git >/dev/null || fail "git is not available"
command -v ssh >/dev/null || fail "ssh is not available"

release_version="$(
  printf '%s\n' "$DEVENV_TASK_INPUT" |
    jq -er '.version | select(type == "string" and length > 0)'
)" || fail "pass --input version=MAJOR.MINOR.PATCH"

if [[ ! "$release_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  fail "version must use the stable MAJOR.MINOR.PATCH form"
fi

release_tag="v$release_version"
current_branch="$(git branch --show-current)"
test "$current_branch" = "main" ||
  fail "releases must be created from main (currently $current_branch)"

test -z "$(git status --porcelain)" ||
  fail "the working tree must be clean; commit or stash changes first"

release_commit_created=false
trap restore_uncommitted_version_change EXIT

printf 'Fetching origin/main and tags...\n'
git fetch --quiet origin --tags

local_head="$(git rev-parse HEAD)"
remote_head="$(git rev-parse refs/remotes/origin/main)"
test "$local_head" = "$remote_head" ||
  fail "local main must exactly match origin/main before preparing a release"

if git show-ref --verify --quiet "refs/tags/$release_tag"; then
  fail "tag $release_tag already exists"
fi

current_version="$(
  cargo metadata --locked --no-deps --format-version 1 |
    jq -er '.packages[] | select(.name == "boxpacker") | .version'
)"

if test "$current_version" != "$release_version"; then
  printf 'Updating Cargo package version from %s to %s...\n' \
    "$current_version" "$release_version"
  cargo set-version "$release_version"
else
  printf 'Cargo package version is already %s.\n' "$release_version"
fi

printf 'Running release checks...\n'
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo build --locked --release

actual_version="$(target/release/boxpacker --version)"
expected_version="boxpacker $release_version"
test "$actual_version" = "$expected_version" ||
  fail "binary reports '$actual_version'; expected '$expected_version'"

if ! git diff --quiet -- Cargo.toml Cargo.lock; then
  git add Cargo.toml Cargo.lock
  git commit -m "Release $release_tag"
  release_commit_created=true
fi

test -z "$(git status --porcelain)" ||
  fail "release checks left unexpected working-tree changes"

git tag -a "$release_tag" -m "BoxPacker $release_tag"

printf 'Pushing main and %s atomically...\n' "$release_tag"
if ! git push --atomic origin main "refs/tags/$release_tag"; then
  git tag -d "$release_tag" >/dev/null
  fail "push failed; the local release commit was kept, but the local tag was removed"
fi

trap - EXIT
printf '\nRelease %s has been dispatched to GitHub Actions.\n' "$release_tag"
