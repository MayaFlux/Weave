# Community module manifest

`community.json` describes a community module and its acquisition requirements.
It does not describe MayaFlux's own dependencies or change the established
framework installation route. Projects remain ordinary CMake directories.

The structural contract is [community.schema.json](community.schema.json)
using JSON Schema Draft 2020-12. Native parsers also check declaration rules
that the schema cannot express. Module creation uses those parsers to validate
metadata.

Module installation and acquisition execution are later phases. Passing
validation does not mean requirements have been installed.

The parser entry points are:

| Platform | File | API |
| -------- | ---- | --- |
| Linux | `linux/lib/community_manifest.py` | `parse_manifest(source, expected_name=None)` |
| macOS | `macos/CommunityManifest.swift` | `CommunityManifest.parse(_:expectedName:)` |
| Windows | `windows/Shared/CommunityManifest.cs` | `CommunityManifest.Parse(source, expectedName)` |

Each parser receives JSON text and an optional expected module name. It returns
a validated document with explicit defaults, or reports a field path and reason.

Parsing does not access the filesystem, inspect installed packages, perform
network operations, or execute commands. Callers will supply operation context
and handle host preconditions and recovery in subsequent phases.

## Revision and compatibility

| Field | Meaning |
| ----- | ------- |
| `schema_version` | Manifest revision. New manifests use `1`. |
| `name` | Required lowercase snake_case module name, beginning with a letter. |
| `min_version` | Required minimum MayaFlux version, such as `0.5.0`. Separate from the schema revision. |
| `needs_lila` | Optional boolean. Missing means false. |
| `licence`, `description` | Optional metadata preserved from existing manifests. |
| `$schema` | Optional editor hint. Weave never fetches or executes its target. |

Existing metadata-only manifests without `schema_version` remain valid. Any
manifest containing `packages`, `manual_installs`, or `assets` must declare
`schema_version: 1`, even when that section is empty.

Missing acquisition sections declare no operations. Unknown fields and
unsupported schema revisions are rejected.

`min_version` has three nonnegative decimal components. Each component is `0`
or a number without a leading zero.

## Package declarations

```json
{
  "schema_version": 1,
  "name": "terrain_field",
  "min_version": "0.5.0",
  "packages": {
    "linux": {
      "arch": {
        "official": ["openimageio"],
        "aur": ["fastnoise2-git"]
      },
      "fedora": {
        "copr": ["author/tools"],
        "packages": ["OpenImageIO-devel", "FastNoise2-devel"]
      },
      "ubuntu": {
        "ppa": ["author/tools"],
        "packages": ["libopenimageio-dev", "fastnoise2-dev"]
      }
    },
    "macos": {
      "brew": {
        "taps": ["author/tools"],
        "formulae": ["openimageio", "author/tools/fastnoise"]
      }
    },
    "windows": {
      "winget": ["Author.Toolchain"],
      "vcpkg": ["openimageio", "fastnoise2"]
    }
  }
}
```

The whole manifest must validate before the active host branch produces
operations. An absent host branch declares no packages for that host. Duplicate
strings within one package list are invalid. Preparation may combine shared
requirements across modules while retaining each module as an owner.

| Declaration | Accepted form |
| ----------- | ------------- |
| Package name | ASCII letters, digits, `+`, `_`, `.`, or `-`, beginning with a letter or digit. |
| Repository | `owner/name`. A PPA omits `ppa:`; Weave supplies it. |
| Homebrew formula | A name that may contain `@`, or `owner/tap/formula`. |
| vcpkg port | Lowercase letters and digits with single separating hyphens. |

These fields contain literal names. They do not accept versions, options,
paths, patterns, or command fragments. vcpkg features and triplets are not
author-supplied command syntax in revision 1.

Before offering an operation, the native manager checks its own identifier
rules. Shape validation cannot establish that a package exists or that a
source is trustworthy. Source additions require separate review, including
native trust or agreement steps. Package-manager recipes may download, build,
or execute code through that manager's normal workflow; show those semantics
in the review.

### Windows sources

A string in `winget` means that exact package ID from the built-in `winget`
source. An object selects another source explicitly:

```json
{
  "schema_version": 1,
  "name": "vendor_tools",
  "min_version": "0.5.0",
  "packages": {
    "windows": {
      "winget_sources": [
        {
          "name": "vendor",
          "url": "https://packages.vendor.example/api",
          "type": "Microsoft.Rest"
        }
      ],
      "winget": [
        { "id": "Vendor.Toolchain", "source": "vendor" }
      ]
    }
  }
}
```

`winget_sources` declares source additions; each addition needs user review.
Supported types are `Microsoft.Rest` and `Microsoft.PreIndexed.Package`.

Custom sources must be declared. The built-in `winget` and `msstore` sources
may be referenced directly, but cannot be redeclared. If a source already
exists under the declared name, its URL and type must match. Preparation fails
on a mismatch and does not replace that source.

Source names and package IDs are literal ASCII identifiers, not search
queries. Source names compare case-insensitively. Package IDs remain exact.

The native community acquisition workflow handles vcpkg discovery, root
selection, bootstrap review, and triplet selection. Authors cannot select an
installation root, inject options, or silently request global integration.

## Manual installation and environment

```json
{
  "schema_version": 1,
  "name": "vendor_sdk_module",
  "min_version": "0.5.0",
  "manual_installs": [
    {
      "id": "vendor_sdk",
      "platforms": ["windows"],
      "url": "https://vendor.example/sdk",
      "description": "Download and install Vendor SDK 4.2 or newer.",
      "verify": { "path": "bin/sdk.exe" },
      "environment": {
        "VENDOR_SDK_ROOT": "{selected_install_directory}",
        "PATH": {
          "value": "{selected_install_directory}/bin",
          "operation": "append_path",
          "scope": "user"
        }
      }
    }
  ]
}
```

### Verification path

The user opens the URL, installs through the vendor's instructions, and selects
the installation directory. `verify.path` is relative to that directory. The
fixed absolute path in the initial task example is superseded by this contract.

Use `/` separators on every platform. Absolute paths and `.` or `..` path
components are invalid. Native resolution must reject links that escape the
selected directory.

Verification checks for a regular file. It never executes that file and does
not prove the installed version. Version requirements remain author
instructions in revision 1.

### Environment values

Variable names use ASCII letters, digits, and underscores, without a leading
digit. A string is shorthand for `operation: set` and `scope: user`. An object
must specify `value`, `operation`, and `scope`.

| Setting | Supported values |
| ------- | ---------------- |
| `operation` | `set`, or `append_path` for one path entry. |
| `scope` | `user` or `project`. Machine-wide changes are excluded. |
| Placeholder | `{selected_install_directory}` only. |

The placeholder refers to the directory selected for that manual installation.

`$HOME`, `%PATH%`, shell expressions, and other environment references remain
literal text. For appended paths, convert `/` separators for the host and
reject values containing its path-list separator. Preserve entry order and
avoid duplicate entries using host path comparison rules.

### Applying environment changes

User scope persists through the host's ordinary user environment mechanism.
Project scope uses an optional, hand-editable standard CMake user preset, with
guidance for terminals and other clients. Show any preset mutation and preserve
existing settings. Existing terminals do not acquire persistent changes
retroactively. MayaFlux's established environment setup stays unchanged.

After verification, review the resolved values, scope, affected file or native
setting, and any overwrites. Conflicting declarations fail preparation until
the user resolves them. Differing scopes must not silently mask incompatible
values. On Windows, variable names compare case-insensitively. Preserve prior
values for recovery.

## Verified assets

```json
{
  "schema_version": 1,
  "name": "terrain_data",
  "min_version": "0.5.0",
  "assets": [
    {
      "id": "terrain_textures",
      "url": "https://example.org/terrain-textures-v1.zip",
      "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
      "size_bytes": 428000000,
      "license_url": "https://example.org/license",
      "destination": "assets/terrain_textures",
      "destination_root": "project",
      "description": "Optional terrain texture collection.",
      "optional": true,
      "archive_format": "zip",
      "max_extracted_size_bytes": 1712000000,
      "max_files": 10000
    }
  ]
}
```

The zero checksum illustrates the required 64-digit hexadecimal shape. It is
not a verified checksum for a real download. Before publishing a module,
replace it with the SHA-256 of the exact archive.

### Destination and limits

`destination_root` selects the project directory or the requesting module's
directory. It defaults to `project`. `destination` is relative to that root.
Asset `platforms` defaults to all three platforms, and `optional` defaults to
false. Declining an optional asset records it as skipped; declining a required
asset leaves its requirement pending.

Revision 1 supports ZIP data archives only.

| Field | Limit |
| ----- | ----- |
| `size_bytes` | Exact archive size and maximum download size. |
| `max_extracted_size_bytes` | Maximum sum of uncompressed file bytes. |
| `max_files` | Maximum archive entries, including directories. |

Each limit is a positive integer no greater than 9007199254740991, so every
client can preserve it exactly. The native client may reject a declared
operation that exceeds host storage capabilities. Review these limits with the
download details before approval.

### Download and extraction

Download into staging and enforce the size limit while streaming. The final
size must match `size_bytes`, and SHA-256 must match before extraction. Keep
HTTPS through redirects. If the origin changes, surface it for review before
continuing. Never execute archive contents.

Inspect every entry before extraction. Reject absolute paths, traversal,
symbolic and hard links, special files, encrypted entries, duplicate or
host-colliding names, and paths resolving outside the approved destination.
For Windows destinations, also reject reserved names, trailing dots or spaces,
and alternate data streams. Account for existing filesystem links. Enforce
declared limits against extracted data, not only archive headers.

Revision 1 never overwrites a destination. Publish a staged tree only when the
destination was absent. On retry, report an existing destination and let the
user resolve it. Failed staging contents can be removed without deleting
pre-existing project or module content.

## Native validation contract

The JSON Schema establishes field shapes. Native declaration validation and
operation preparation also have responsibilities that the schema cannot express.

### When parsing a declaration

- Reject duplicate JSON object keys and nonstandard numbers. Accept a leading
  UTF-8 byte-order mark. Limit object and array nesting to 64 containers.
- Require integer JSON tokens for numeric fields. Fraction and exponent
  notation are invalid, even when they represent a mathematical integer.
  This prevents native decoders from rounding a value into acceptance.
- Apply defaults explicitly. JSON Schema `default` annotations do not insert
  values into documents.
- Match the manifest name to the requested module. Validate every branch,
  including inactive platforms, before acquisition.
- Require unique manual installation IDs and unique asset IDs within their
  respective lists. IDs are scoped to their module.
- Validate HTTPS URLs with a native URL parser. Require a hostname and reject
  credentials, fragments, control characters, and invalid ports. The schema
  pattern is not a complete URL parser. Licence and manual URLs are displayed
  or opened, never treated as installer commands.

### When preparing operations

- Check package-manager syntax, custom source references, duplicate source
  names, built-in source collisions, and conflicts across modules. A source
  reference must name a built-in or declared source.
- Resolve paths against their declared roots. Check host filename rules and
  filesystem containment. Report inaccessible paths as precondition failures.
  Lexical path validation alone does not establish containment.
- Validate resolved environment changes and report conflicts before applying
  changes. Never evaluate author text as shell or process syntax.
- Require the installed MayaFlux version to satisfy both registry and manifest
  minimum versions. The higher minimum applies. A missing or malformed
  installed version blocks compatibility-dependent registration with a
  diagnostic; it does not trigger framework installation.
- Return errors with a JSON field path, reason, affected module, and recovery
  choice. Distinguish invalid declarations from unavailable tools, cancelled
  review, and execution failures. Never partially accept an invalid manifest.

Validation does not inspect module CMake as executable configuration. Authors
must keep their CMake requirements consistent with the manifest. Existing
build-time CMake checks remain in place, and acquisition does not rewrite
user-authored CMake.

Module authors are responsible for dependencies, sources, licences, and
compatibility. Weave displays and executes approved typed operations.
Heuristic flags can surface external sources, downloads, large assets, or
compiler and dynamic-library path changes. They are not a safety guarantee.

## References

- [JSON Schema validation](https://json-schema.org/draft/2020-12/json-schema-validation)
- [WinGet sources](https://learn.microsoft.com/en-us/windows/package-manager/winget/source)
- [vcpkg install](https://learn.microsoft.com/en-us/vcpkg/commands/install)
- [vcpkg CMake integration](https://learn.microsoft.com/en-us/vcpkg/users/buildsystems/cmake-integration)
