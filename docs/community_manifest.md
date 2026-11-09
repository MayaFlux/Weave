# Community module manifest

Put `community.json` in the root of your module repository, beside
`<module_name>.cmake` and `src/`. It describes the module, its minimum MayaFlux
version, and any additional requirements users need to complete.

The structural contract is [community.schema.json](community.schema.json)
using JSON Schema Draft 2020-12. Weave also checks declaration rules that the
schema cannot express. Choose **Create Community Module** or run
`Weave community <module_name>` to generate a manifest with basic metadata,
then add the declarations your module needs.

When a user adds a module, Weave checks its files, manifest, and MayaFlux
compatibility before registration in `community.cmake`.

| Declaration | What Weave does when adding the module |
| --- | --- |
| Metadata and `min_version` | Validates the manifest and checks the installed MayaFlux version. |
| `urls` | Asks before opening each applicable URL and displays any destination hint. |
| `packages` | Validates declarations and tells the user to review the manifest. |
| `manual_installs` and `assets` | Validates declarations and leaves installation or downloads to the user. |

Weave does not install declared community packages, add package sources, apply
declared environment changes, or download and extract declared assets. Include
the necessary user instructions in your module's README. Passing manifest
validation or opening a browser does not establish that a requirement is ready.

## Revision and compatibility

| Field | Meaning |
| ----- | ------- |
| `schema_version` | Manifest revision. Use `1` for acquisition declarations. |
| `name` | Required lowercase snake_case module name, beginning with a letter. |
| `min_version` | Required minimum MayaFlux version, such as `0.5.0`. Separate from the schema revision. |
| `needs_lila` | Optional boolean. Missing means false. |
| `licence`, `description` | Optional metadata preserved from existing manifests. |
| `$schema` | Optional editor hint. Weave never fetches or executes its target. |

Existing metadata-only manifests without `schema_version` remain valid. Any
manifest containing acquisition sections must declare a schema revision.
The current revision is `1`, including the `urls` field.

Missing acquisition sections declare no operations. Unknown fields and
unsupported schema revisions are rejected.

`min_version` has three nonnegative decimal components. Each component is `0`
or a number without a leading zero.

## URLs

Use the `urls` array for download pages, files, account setup, or other browser
actions. Only `url` is required for each entry:

```json
{
  "schema_version": 1,
  "name": "terrain_field",
  "min_version": "0.6.0",
  "urls": [
    {
      "url": "https://example.org/download",
      "description": "Open the provider's download page and follow its instructions."
    },
    {
      "url": "https://example.org/data/model.bin",
      "destination": "data/model.bin",
      "destination_root": "module"
    }
  ]
}
```

A URL can open in the user's browser for any action the site requires. Weave
asks for permission before each browser launch in the GUI or CLI. If the
user declines or the browser cannot open, Weave reports the URL for manual
use. Browser handoff lets the browser apply its site and download warnings.
Weave continues module addition as soon as it hands the URL to the browser;
it does not wait for the page or download. Opening a URL does not establish
that any site action or download is complete. A URL can point to any web
content, including a file or a page that requires user interaction.

`destination` is an optional relative file path telling the user where to
place a downloaded file. `destination_root` selects `project` (the default)
or `module`. Weave does not write or overwrite the destination. If a file
already exists, Weave reports it for review and does not assume its contents
match the URL. `platforms` optionally limits resolution to `linux`, `macos`,
and `windows`; absent means all three. Browser handoffs and missing files
remain pending user actions while module registration proceeds.

The URL must use HTTPS and contain no embedded credentials. URL fragments are
allowed for links to pages. Weave rechecks the URL before browser handoff.
The browser handles certificate validation and any configured protection
against known malicious sites and downloads. Weave does not claim a malware
verdict or treat browser launch as proof of completed user actions. A URL may
lead to a page that needs login or other user steps. The declaration does not
promise that a transfer yields a file or that site actions can be automated.

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

An absent host branch declares no packages for that host. Duplicate strings
within one package list are invalid. Weave validates every platform branch,
even when adding the module on another OS. Users install these requirements
themselves using your instructions.

| Declaration | Accepted form |
| ----------- | ------------- |
| Package name | ASCII letters, digits, `+`, `_`, `.`, or `-`, beginning with a letter or digit. |
| Repository | `owner/name`. A PPA omits `ppa:`; Weave supplies it. |
| Homebrew formula | A name that may contain `@`, or `owner/tap/formula`. |
| vcpkg port | Lowercase letters and digits with single separating hyphens. |

These fields contain literal names. They do not accept versions, options,
paths, patterns, or command fragments. vcpkg features and triplets are not
author-supplied command syntax in revision 1.

An accepted identifier does not establish that a package exists. Tell users
which package manager to use and provide any required source setup instructions.

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

`winget_sources` describes additional sources for the user to configure.
Supported types are `Microsoft.Rest` and `Microsoft.PreIndexed.Package`.

Custom sources must be declared. The built-in `winget` and `msstore` sources
may be referenced directly, but cannot be redeclared. Weave checks source
references within the manifest; it does not inspect or change the user's
configured WinGet sources.

Source names and package IDs are literal ASCII identifiers, not search
queries. Source names compare case-insensitively. Package IDs remain exact.

For vcpkg requirements, document the installation and triplet your module
expects. Weave does not bootstrap or configure vcpkg for community modules.

## Manual installation and environment declarations

These declarations describe requirements for users to handle manually. Weave
does not show an installation-directory picker or apply the environment values.

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

Document how to install the requirement and locate its installation directory.
`verify.path` names the expected file relative to that directory. Weave validates
the path declaration but does not check that file during module addition.

Use `/` separators on every platform. Absolute paths and `.` or `..` path
components are invalid.

Include any required version checks in your user instructions. The presence of
an expected file alone does not establish its version.

### Environment values

Variable names use ASCII letters, digits, and underscores, without a leading
digit. A string is shorthand for `operation: set` and `scope: user`. An object
must specify `value`, `operation`, and `scope`.

| Setting | Supported values |
| ------- | ---------------- |
| `operation` | `set`, or `append_path` for one path entry. |
| `scope` | `user` or `project`. Machine-wide changes are excluded. |
| Placeholder | `{selected_install_directory}` only. |

The placeholder represents the installation directory described in your
instructions. Weave does not expand it during module addition.

`$HOME`, `%PATH%`, shell expressions, and other environment references remain
literal text in the declaration. Show users the actual values to set and the
appropriate path separators for their OS.

## Asset declarations

Asset metadata records the expected archive and destination. Users download
and extract it themselves; Weave does not perform the operations described by
these fields. Use `urls` as well if you want to offer a browser link.

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
false. Explain in your README whether users need the asset before building or
running the module.

Revision 1 supports ZIP data archives only.

| Field | Limit |
| ----- | ----- |
| `size_bytes` | Exact archive size and maximum download size. |
| `max_extracted_size_bytes` | Maximum sum of uncompressed file bytes. |
| `max_files` | Maximum archive entries, including directories. |

Each limit is a positive integer no greater than 9007199254740991, so every
client can preserve it exactly. These are validated declarations; Weave does
not enforce them against a download or extracted files.

## Validation rules

Correct a rejected declaration using the field path and reason in Weave's
diagnostic. The parser applies these rules in addition to the schema:

- Reject duplicate JSON object keys and nonstandard numbers. Accept a leading
  UTF-8 byte-order mark. Limit object and array nesting to 64 containers.
- Require integer JSON tokens for numeric fields. Fraction and exponent
  notation are invalid, even when they represent a mathematical integer.
  This prevents native decoders from rounding a value into acceptance.
- Apply defaults explicitly. JSON Schema `default` annotations do not insert
  values into documents.
- Match the manifest name to the requested module. Validate every branch,
  including inactive platforms, before registration.
- Require unique manual installation IDs and unique asset IDs within their
  respective lists. IDs are scoped to their module.
- Validate HTTPS URLs with a native URL parser. Require a hostname and reject
  credentials, control characters, and invalid ports. Fragments are allowed
  only in `urls` entries. The schema pattern is not a complete URL parser.
- Require package-source references to name a built-in or declared source.
- Reject unsafe relative paths and invalid Windows filenames in declarations
  that apply to Windows.

The installed MayaFlux version must satisfy both the registry minimum and the
manifest's `min_version`. If compatibility cannot be established, module
addition reports the problem rather than installing MayaFlux automatically.

Validation does not inspect module CMake as executable configuration. Authors
must keep their CMake requirements consistent with the manifest. Existing
build-time CMake checks remain in place, and acquisition does not rewrite
user-authored CMake.

Module authors are responsible for dependencies, sources, licences, and
compatibility. Keep the manifest and your installation instructions in sync.

## References

- [JSON Schema validation](https://json-schema.org/draft/2020-12/json-schema-validation)
- [WinGet sources](https://learn.microsoft.com/en-us/windows/package-manager/winget/source)
- [vcpkg install](https://learn.microsoft.com/en-us/vcpkg/commands/install)
- [vcpkg CMake integration](https://learn.microsoft.com/en-us/vcpkg/users/buildsystems/cmake-integration)
