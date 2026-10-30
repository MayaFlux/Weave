#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="$(sed -n 's/^version = "\([0-9][0-9.]*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
fail() { echo "Weave release: $*" >&2; exit 1; }
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail 'invalid Cargo package version'

plan() {
    [[ "${GITHUB_SHA:-}" =~ ^[0-9a-f]{40}$ ]] || fail 'GITHUB_SHA must name the built commit'
    [[ "${GITHUB_REPOSITORY:-}" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || fail 'GITHUB_REPOSITORY is required'
    case "${GITHUB_REF:-}" in
        refs/tags/*)
            TAG="${GITHUB_REF#refs/tags/}"
            [[ "$TAG" == "v$VERSION" || "$TAG" == "v$VERSION-dev" ]] || fail "tag $TAG does not match Cargo version $VERSION"
            ;;
        refs/heads/main) TAG="v$VERSION" ;;
        refs/heads/*) TAG="v$VERSION-dev" ;;
        *) fail 'unsupported release ref' ;;
    esac
    PRERELEASE=false
    [[ "$TAG" != *-dev ]] || PRERELEASE=true
}

assemble() {
    local packages="${1:?Package directory required}" destination="${2:?Release directory required}"
    local macos="$packages/Weave-$VERSION-macOS-arm64.dmg"
    local linux="$packages/Weave-$VERSION-Linux-x86_64.AppImage"
    local windows="$packages/Weave-$VERSION-Windows-x64.exe"
    for artifact in "$macos" "$linux" "$windows"; do
        [[ -s "$artifact" && ! -L "$artifact" ]] || fail "missing validated platform artifact: $artifact"
    done
    plan
    local download="https://github.com/$GITHUB_REPOSITORY/releases/download/$TAG"
    mkdir -p "$destination"
    cp "$macos" "$destination/Weave-macos.dmg"
    cp "$linux" "$destination/Weave-linux.AppImage"
    cp "$windows" "$destination/Weave-windows.exe"
    (cd "$destination" && sha256sum Weave-macos.dmg Weave-linux.AppImage Weave-windows.exe > SHA256SUMS)
    cat > "$destination/release-notes.md" <<EOF
# Weave $TAG

Install MayaFlux, create projects and community modules, and update your projects.
Use the desktop app or the standalone CLI.

## Downloads

| Platform | Download | Requirements |
| --- | --- | --- |
| Windows x64 | [Executable]($download/Weave-windows.exe) | Windows 10 22H2 (build 19045) or later |
| macOS Apple Silicon | [DMG]($download/Weave-macos.dmg) | arm64, macOS 26 or later |
| Linux x86_64 | [AppImage]($download/Weave-linux.AppImage) | glibc 2.43 or later, host FUSE |

On Windows, download and open **Weave-windows.exe** as an ordinary user. Templates
and scripts are embedded. No installer or separate Weave runtime is required.
The same executable provides the CLI: \`Weave-windows.exe --help\`.

On macOS, open the DMG and copy **Weave.app** to Applications. An ad-hoc signature
does not satisfy Gatekeeper for browser downloads. If macOS blocks this build,
use **System Settings > Privacy & Security > Open Anyway** to approve it.
The app also includes its CLI and resources.

On Linux, make the AppImage executable and launch it. **Host FUSE is required.**
All platforms use the host display and GPU driver. MayaFlux, Git, CMake,
compilers and other project tools remain separate host dependencies. Host
installation requests native authorization when needed; project and community
operations do not require elevation.

Choose a stable release for regular use or a development prerelease to try
the latest changes. Development prereleases are refreshed with successful builds.

## SHA256

Download [SHA256SUMS]($download/SHA256SUMS) to verify the files:

\`\`\`text
$(cat "$destination/SHA256SUMS")
\`\`\`

Built from commit \`$GITHUB_SHA\`.
EOF
}

publish() {
    local directory="${1:?Release directory required}" tag_sha existing_draft existing_assets exists=false
    local obsolete_windows_zip=false
    [[ "${GITHUB_ACTIONS:-}" == true ]] || fail 'publishing is only run by the release job'
    plan
    for artifact in Weave-macos.dmg Weave-linux.AppImage Weave-windows.exe SHA256SUMS release-notes.md; do
        [[ -s "$directory/$artifact" && ! -L "$directory/$artifact" ]] || fail "missing release input: $artifact"
    done
    (cd "$directory" && sha256sum --check SHA256SUMS) || fail 'release asset checksum mismatch'
    tag_sha="$(git rev-parse --verify "$TAG^{commit}" 2>/dev/null || true)"
    if [[ "$PRERELEASE" == false && -n "$tag_sha" && "$tag_sha" != "$GITHUB_SHA" ]]; then
        [[ "$GITHUB_REF" == refs/heads/main ]] || fail 'stable tag does not point to the built commit'
        echo "::notice::Stable $TAG already identifies another commit. Packages are uploaded as CI artifacts; bump Cargo.toml to publish a new stable version."
        return
    fi
    # Only a successful complete lookup can establish that a release is absent.
    # An authentication/network/API failure must stop before moving any tag.
    existing_draft="$(gh api --paginate "repos/$GITHUB_REPOSITORY/releases?per_page=100" --jq ".[] | select(.tag_name == \"$TAG\") | .draft")" || fail 'cannot look up existing releases; no tag was moved'
    if [[ -n "$existing_draft" ]]; then
        [[ "$existing_draft" == true || "$existing_draft" == false ]] || fail 'unexpected release lookup response'
        exists=true
        if [[ "$PRERELEASE" == false && "$existing_draft" == false ]]; then
            echo "Stable release $TAG is already published; preserving its assets."
            return
        fi
        existing_assets="$(gh release view "$TAG" --json assets --jq '.assets[].name')" || fail 'cannot look up existing release assets; no tag was moved'
        if grep -Fxq 'Weave-windows.zip' <<< "$existing_assets"; then obsolete_windows_zip=true; fi
        # Keep incomplete uploads out of the published release. A failed run
        # leaves a draft that can be repaired by rerunning the same build.
        gh release edit "$TAG" --draft=true
    fi
    if [[ "$PRERELEASE" == true ]]; then
        git tag --force "$TAG" "$GITHUB_SHA"
        git push origin "+refs/tags/$TAG:refs/tags/$TAG"
    elif [[ -z "$tag_sha" ]]; then
        git tag "$TAG" "$GITHUB_SHA"
        git push origin "refs/tags/$TAG:refs/tags/$TAG"
    fi
    if [[ "$exists" == false ]]; then
        gh release create "$TAG" --verify-tag --draft --title "Weave $TAG" --notes-file "$directory/release-notes.md"
    fi
    gh release upload "$TAG" "$directory/Weave-macos.dmg" "$directory/Weave-linux.AppImage" "$directory/Weave-windows.exe" "$directory/SHA256SUMS" --clobber
    if [[ "$obsolete_windows_zip" == true ]]; then
        gh release delete-asset "$TAG" Weave-windows.zip --yes
    fi
    local latest=true
    [[ "$PRERELEASE" != true ]] || latest=false
    gh release edit "$TAG" --title "Weave $TAG" --notes-file "$directory/release-notes.md" --prerelease="$PRERELEASE" --latest="$latest" --draft=false
}

case "${1:-}" in
    plan) plan; printf 'version=%s\ntag=%s\nprerelease=%s\n' "$VERSION" "$TAG" "$PRERELEASE" ;;
    assemble) assemble "${2:-}" "${3:-}" ;;
    publish) publish "${2:-}" ;;
    *) fail 'usage: release.sh plan | assemble <packages> <release> | publish <release>' ;;
esac
