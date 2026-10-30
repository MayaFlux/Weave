#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$ROOT/packaging/scripts/release.sh"
TEMP_ROOT="$(cd "${TMPDIR:-/tmp}" && pwd)"
WORK="$(mktemp -d "$TEMP_ROOT/weave-release-test.XXXXXX")"
WORK="$(cd "$WORK" && pwd)"
[[ "$WORK" == "$TEMP_ROOT"/weave-release-test.* ]] || exit 1
trap 'rm -rf "$WORK"' EXIT
VERSION="$(sed -n 's/^version = "\([0-9][0-9.]*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
mkdir -p "$WORK/bin" "$WORK/packages"
export GITHUB_SHA=0123456789012345678901234567890123456789
export GITHUB_REPOSITORY=MayaFlux/Weave
export GITHUB_ACTIONS=true
export FAKE_LOG="$WORK/calls"
export FAKE_TAG_SHA="" FAKE_RELEASE_EXISTS=0 FAKE_RELEASE_DRAFT=false FAKE_FAIL_UPLOAD=0 FAKE_LOOKUP_FAIL=0
cat > "$WORK/bin/git" <<'EOF'
#!/usr/bin/env bash
if [[ "$1" == rev-parse ]]; then
    [[ -n "$FAKE_TAG_SHA" ]] || exit 1
    printf '%s\n' "$FAKE_TAG_SHA"
else
    printf 'git %s\n' "$*" >> "$FAKE_LOG"
fi
EOF
cat > "$WORK/bin/gh" <<'EOF'
#!/usr/bin/env bash
printf 'gh %s\n' "$*" >> "$FAKE_LOG"
if [[ "$1" == api ]]; then
    [[ "$FAKE_LOOKUP_FAIL" == 0 ]] || exit 1
    if [[ "$FAKE_RELEASE_EXISTS" == 1 ]]; then printf '%s\n' "$FAKE_RELEASE_DRAFT"; fi
    exit 0
fi
if [[ "$1 $2" == 'release upload' && "$FAKE_FAIL_UPLOAD" == 1 ]]; then exit 1; fi
EOF
chmod +x "$WORK/bin/git" "$WORK/bin/gh"
export PATH="$WORK/bin:$PATH"
expect() { grep -Fq -- "$1" "$FAKE_LOG" || { echo "Missing expected release action: $1" >&2; cat "$FAKE_LOG" >&2; exit 1; }; }
reject() { if grep -Fq -- "$1" "$FAKE_LOG"; then echo "Unexpected release action: $1" >&2; exit 1; fi; }

export GITHUB_REF=refs/heads/develop
bash "$SCRIPT" plan > "$WORK/plan"
grep -Fxq "tag=v$VERSION-dev" "$WORK/plan"
grep -Fxq 'prerelease=true' "$WORK/plan"
GITHUB_REF=refs/heads/main bash "$SCRIPT" plan > "$WORK/plan"
grep -Fxq "tag=v$VERSION" "$WORK/plan"
grep -Fxq 'prerelease=false' "$WORK/plan"
GITHUB_REF="refs/tags/v$VERSION-dev" bash "$SCRIPT" plan > "$WORK/plan"
grep -Fxq 'prerelease=true' "$WORK/plan"
if GITHUB_REF=refs/tags/not-a-version bash "$SCRIPT" plan >/dev/null 2>&1; then echo 'Mismatched version tag was accepted' >&2; exit 1; fi

for name in "Weave-$VERSION-macOS-arm64.dmg" "Weave-$VERSION-Linux-x86_64.AppImage" "Weave-$VERSION-Windows-x64.zip"; do
    printf 'controlled fixture %s\n' "$name" > "$WORK/packages/$name"
done
bash "$SCRIPT" assemble "$WORK/packages" "$WORK/release"
(cd "$WORK/release" && sha256sum --check SHA256SUMS)
grep -Fq 'Windows 10 22H2' "$WORK/release/release-notes.md"
grep -Fq 'Open Anyway' "$WORK/release/release-notes.md"
if grep -Eiq '\.NET|as administrator|no CLI' "$WORK/release/release-notes.md"; then echo 'Legacy distribution instructions leaked into release notes' >&2; exit 1; fi

# A rolling development build advances its tag and replaces assets as a draft,
# then publishes only after every upload succeeds. These commands are fakes.
export FAKE_TAG_SHA=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa FAKE_RELEASE_EXISTS=1
: > "$FAKE_LOG"
bash "$SCRIPT" publish "$WORK/release"
expect "git tag --force v$VERSION-dev"
expect "git push origin +refs/tags/v$VERSION-dev:refs/tags/v$VERSION-dev"
expect '--draft=true'
expect "gh release upload v$VERSION-dev"
expect '--clobber'
expect '--prerelease=true --latest=false --draft=false'

export FAKE_FAIL_UPLOAD=1
: > "$FAKE_LOG"
if bash "$SCRIPT" publish "$WORK/release" >/dev/null 2>&1; then echo 'Failed upload was accepted' >&2; exit 1; fi
reject '--draft=false'
export FAKE_FAIL_UPLOAD=0

export FAKE_LOOKUP_FAIL=1
: > "$FAKE_LOG"
if bash "$SCRIPT" publish "$WORK/release" >/dev/null 2>&1; then echo 'Failed release lookup was accepted' >&2; exit 1; fi
reject 'git tag'
reject 'git push'
reject 'gh release edit'
export FAKE_LOOKUP_FAIL=0

# Stable releases never move an existing version to another commit.
: > "$FAKE_LOG"
GITHUB_REF=refs/heads/main bash "$SCRIPT" publish "$WORK/release"
reject 'git tag'
reject 'git push'
reject 'gh release upload'

export FAKE_TAG_SHA="" FAKE_RELEASE_EXISTS=0
: > "$FAKE_LOG"
GITHUB_REF=refs/heads/main bash "$SCRIPT" publish "$WORK/release"
expect "git tag v$VERSION $GITHUB_SHA"
expect "git push origin refs/tags/v$VERSION:refs/tags/v$VERSION"
reject 'git tag --force'
reject '+refs/tags'

export FAKE_TAG_SHA="$GITHUB_SHA" FAKE_RELEASE_EXISTS=0
: > "$FAKE_LOG"
GITHUB_REF="refs/tags/v$VERSION" bash "$SCRIPT" publish "$WORK/release"
reject 'git tag'
reject 'git push'
expect "gh release create v$VERSION"
expect '--verify-tag --draft'
expect '--prerelease=false --latest=true --draft=false'

export FAKE_RELEASE_EXISTS=1
: > "$FAKE_LOG"
GITHUB_REF="refs/tags/v$VERSION" bash "$SCRIPT" publish "$WORK/release"
reject 'git tag'
reject 'git push'
reject 'gh release upload'
reject 'gh release edit'

# A missing/tampered platform package must fail before any remote mutation.
printf tampered >> "$WORK/release/Weave-windows.zip"
: > "$FAKE_LOG"
if bash "$SCRIPT" publish "$WORK/release" >/dev/null 2>&1; then echo 'Tampered assets were accepted' >&2; exit 1; fi
[[ ! -s "$FAKE_LOG" ]]
mkdir "$WORK/incomplete"
cp "$WORK/packages/Weave-$VERSION-macOS-arm64.dmg" "$WORK/incomplete/"
if bash "$SCRIPT" assemble "$WORK/incomplete" "$WORK/incomplete-release" >/dev/null 2>&1; then echo 'Partial release was accepted' >&2; exit 1; fi
echo 'Release policy, assembly, checksums, rolling tags and upload failure tests passed.'
