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
    FORCE_RERELEASE="${WEAVE_FORCE_RERELEASE:-false}"
    [[ "$FORCE_RERELEASE" == true || "$FORCE_RERELEASE" == false ]] || fail 'invalid force re-release option'
    if [[ "$FORCE_RERELEASE" == true ]]; then
        [[ "${GITHUB_EVENT_NAME:-}" == workflow_dispatch && "$GITHUB_REF" == refs/heads/main ]] || fail 'force re-release requires a manual workflow run on main'
    fi
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
    for document in README.md docs/community_manifest.md docs/develop.md; do
        [[ -s "$ROOT/$document" ]] || fail "missing release guide: $document"
    done
    local download="https://github.com/$GITHUB_REPOSITORY/releases/download/$TAG"
    local guide="https://github.com/$GITHUB_REPOSITORY/blob/$GITHUB_SHA"
    mkdir -p "$destination"
    cp "$macos" "$destination/Weave-macos.dmg"
    cp "$linux" "$destination/Weave-linux.AppImage"
    cp "$windows" "$destination/Weave-windows.exe"
    (cd "$destination" && sha256sum Weave-macos.dmg Weave-linux.AppImage Weave-windows.exe > SHA256SUMS)
    cat > "$destination/release-notes.md" <<EOF
# Weave $TAG

Install MayaFlux, create projects and community modules, and manage existing projects.
Use the desktop app or the standalone CLI.

## Downloads

| Platform | Download | Requirements |
| --- | --- | --- |
| Windows x64 | [Executable]($download/Weave-windows.exe) | Windows 10 22H2 (build 19045) or later |
| macOS Apple Silicon | [DMG]($download/Weave-macos.dmg) | arm64, macOS 26 or later |
| Linux x86_64 | [AppImage]($download/Weave-linux.AppImage) | glibc 2.43 or later, host FUSE |

On Windows, download and open **Weave-windows.exe** as an ordinary user. Templates
and scripts are embedded. No installer or separate Weave runtime is required.
The same executable provides the CLI: \`.\Weave-windows.exe --help\`.

On macOS, open the DMG and copy **Weave.app** to Applications. If macOS blocks
this download, use **System Settings > Privacy & Security > Open Anyway** to approve it.
The app also includes its CLI and resources.

On Linux, enable execution in the AppImage's file properties if needed, then open it.
Alternatively, run \`chmod +x Weave-linux.AppImage\`, then \`./Weave-linux.AppImage\`.
**Host FUSE is required.**
All platforms use the host display and GPU driver. MayaFlux, Git, CMake,
compilers and other project tools remain separate host dependencies. Host
installation requests native authorization when needed; project and community
operations do not require elevation.

## Using Weave

1. Choose **Install MayaFlux**, select **Stable** or **Development**, and follow
   the setup prompts. Leave CLI setup selected to use \`Weave\` in new terminals.
2. Choose **Projects > Create Project**, select its name and destination, then
   build from the project folder with \`cmake --preset release\` and
   \`cmake --build --preset release\`.
3. Choose **Projects > Manage Project** and select an existing project to add
   Lila live coding, copy shader sources, add community modules, or validate it.
   Community URLs open only after your confirmation. Complete any requested
   browser steps or downloads yourself; Weave reports pending destinations.
4. Use **Run & Report** after building to reproduce a problem. Choose **Stop and collect**
   for normal shutdown or **Force stop** if the app does not respond. Attach
   \`mayaflux_run.txt\` and the application report if it was created.
5. Use **Refresh Project Files** to update older project scripts. Review the
   selected files and uncheck your own code or settings before applying.
   Replaced files are backed up under \`.weave-refresh-backups/\`.

Weave development prereleases are refreshed with successful full manual builds.
The MayaFlux stream is selected separately inside Weave.

## Guides for this build

- [User guide and platform setup]($guide/README.md)
- [Community module manifest]($guide/docs/community_manifest.md)
- [Building and contributing to Weave]($guide/docs/develop.md)

These links use the built commit, including for rolling development releases.

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
    [[ "${GITHUB_EVENT_NAME:-}" == workflow_dispatch ]] || fail 'publishing requires a manual workflow run'
    plan
    for artifact in Weave-macos.dmg Weave-linux.AppImage Weave-windows.exe SHA256SUMS release-notes.md; do
        [[ -s "$directory/$artifact" && ! -L "$directory/$artifact" ]] || fail "missing release input: $artifact"
    done
    (cd "$directory" && sha256sum --check SHA256SUMS) || fail 'release asset checksum mismatch'
    tag_sha="$(git rev-parse --verify "$TAG^{commit}" 2>/dev/null || true)"
    if [[ "$PRERELEASE" == false && "$FORCE_RERELEASE" == false && -n "$tag_sha" && "$tag_sha" != "$GITHUB_SHA" ]]; then
        [[ "$GITHUB_REF" == refs/heads/main ]] || fail 'stable tag does not point to the built commit'
        echo "::notice::Stable $TAG already identifies another commit. Use a manual main run with Force re-release to replace this version, or bump Cargo.toml for a new version."
        return
    fi
    # Only a successful complete lookup can establish that a release is absent.
    # An authentication/network/API failure must stop before moving any tag.
    existing_draft="$(gh api --paginate "repos/$GITHUB_REPOSITORY/releases?per_page=100" --jq ".[] | select(.tag_name == \"$TAG\") | .draft")" || fail 'cannot look up existing releases; no tag was moved'
    if [[ -n "$existing_draft" ]]; then
        [[ "$existing_draft" == true || "$existing_draft" == false ]] || fail 'unexpected release lookup response'
        exists=true
        if [[ "$PRERELEASE" == false && "$FORCE_RERELEASE" == false && "$existing_draft" == false ]]; then
            echo "Stable release $TAG is already published; preserving its assets."
            return
        fi
        existing_assets="$(gh release view "$TAG" --json assets --jq '.assets[].name')" || fail 'cannot look up existing release assets; no tag was moved'
        if grep -Fxq 'Weave-windows.zip' <<< "$existing_assets"; then obsolete_windows_zip=true; fi
        # Keep incomplete uploads out of the published release. A failed run
        # leaves a draft that can be repaired by rerunning the same build.
        gh release edit "$TAG" --draft=true
    fi
    if [[ "$PRERELEASE" == true || "$FORCE_RERELEASE" == true ]]; then
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
