//! Shared parser and validator for community module manifests.

use serde::Deserializer;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError {
    pub path: String,
    pub reason: String,
}

impl Display for ManifestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.path, self.reason)
    }
}

impl std::error::Error for ManifestError {}

#[derive(Debug, Clone)]
pub struct CommunityManifest {
    pub minimum_version: String,
    pub data: Value,
}

pub fn parse_manifest(
    source: &str,
    expected_name: &str,
) -> Result<CommunityManifest, ManifestError> {
    let source = source.trim_start_matches('\u{feff}');
    let mut deserializer = serde_json::Deserializer::from_str(source);
    let mut data = StrictJsonSeed {
        depth: 0,
        path: "$".to_owned(),
    }
    .deserialize(&mut deserializer)
    .map_err(|error| parse_error(&error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| parse_error(&error.to_string()))?;
    let minimum_version = validate_manifest(&data, expected_name)?;
    insert_defaults(&mut data);
    Ok(CommunityManifest {
        minimum_version,
        data,
    })
}

fn error(path: &str, reason: impl Into<String>) -> ManifestError {
    ManifestError {
        path: path.to_owned(),
        reason: reason.into(),
    }
}

fn parse_error(message: &str) -> ManifestError {
    if let Some((path, reason)) = message.split_once(": ")
        && path.starts_with('$')
    {
        return error(path, reason.split(" at line ").next().unwrap_or(reason));
    }
    error("$", format!("Invalid JSON: {message}"))
}

struct StrictJsonSeed {
    depth: usize,
    path: String,
}

impl<'de> DeserializeSeed<'de> for StrictJsonSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictJsonVisitor {
            depth: self.depth,
            path: self.path,
        })
    }
}

struct StrictJsonVisitor {
    depth: usize,
    path: String,
}

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Value, E>
    where
        E: de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom(format!("{}: Nonstandard or nonfinite number", self.path)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        if self.depth >= 64 {
            return Err(de::Error::custom(format!(
                "{}: JSON nesting exceeds supported depth",
                self.path
            )));
        }
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictJsonSeed {
            depth: self.depth + 1,
            path: format!("{}[{}]", self.path, values.len()),
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        if self.depth >= 64 {
            return Err(de::Error::custom(format!(
                "{}: JSON nesting exceeds supported depth",
                self.path
            )));
        }
        let mut values = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!(
                    "{}.{}: Duplicate object key",
                    self.path, key
                )));
            }
            let child = map.next_value_seed(StrictJsonSeed {
                depth: self.depth + 1,
                path: format!("{}.{}", self.path, key),
            })?;
            values.insert(key, child);
        }
        Ok(Value::Object(values))
    }
}

const TOP_FIELDS: &[&str] = &[
    "$schema",
    "schema_version",
    "name",
    "min_version",
    "needs_lila",
    "licence",
    "description",
    "packages",
    "manual_installs",
    "assets",
];

fn validate_manifest(data: &Value, expected_name: &str) -> Result<String, ManifestError> {
    let object = object(data, "$", TOP_FIELDS, &["name", "min_version"])?;
    let name = text(required(object, "name", "$")?, "$.name", true)?;
    if !valid_identifier(name) {
        return Err(error("$.name", "Invalid identifier"));
    }
    if name != expected_name {
        return Err(error(
            "$.name",
            "Manifest name does not match requested module",
        ));
    }
    let minimum = text(required(object, "min_version", "$")?, "$.min_version", true)?;
    if !valid_version(minimum) {
        return Err(error("$.min_version", "Expected major.minor.patch version"));
    }
    let minimum_version = minimum.to_owned();

    if let Some(revision) = object.get("schema_version") {
        if revision.as_u64() != Some(1) {
            return Err(error("$.schema_version", "Unsupported schema revision"));
        }
    } else if ["packages", "manual_installs", "assets"]
        .iter()
        .any(|field| object.contains_key(*field))
    {
        return Err(error(
            "$.schema_version",
            "Acquisition declarations require schema revision 1",
        ));
    }

    for field in ["$schema", "licence", "description"] {
        if let Some(value) = object.get(field) {
            text(value, &format!("$.{field}"), field == "$schema")?;
        }
    }
    if let Some(value) = object.get("needs_lila") {
        boolean(value, "$.needs_lila")?;
    }
    if let Some(packages) = object.get("packages") {
        validate_packages(packages)?;
    }
    validate_requirement_list(
        object.get("manual_installs"),
        "manual_installs",
        validate_manual,
    )?;
    validate_requirement_list(object.get("assets"), "assets", validate_asset)?;
    Ok(minimum_version)
}

fn object<'a>(
    value: &'a Value,
    path: &str,
    allowed: &[&str],
    required: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, ManifestError> {
    let Some(value) = value.as_object() else {
        return Err(error(path, "Expected object"));
    };
    for key in value.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(error(&format!("{path}.{key}"), "Unknown field"));
        }
    }
    for key in required {
        if !value.contains_key(*key) {
            return Err(error(&format!("{path}.{key}"), "Required field is missing"));
        }
    }
    Ok(value)
}

fn required<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<&'a Value, ManifestError> {
    object
        .get(key)
        .ok_or_else(|| error(&format!("{path}.{key}"), "Required field is missing"))
}

fn text<'a>(value: &'a Value, path: &str, nonempty: bool) -> Result<&'a str, ManifestError> {
    let Some(value) = value.as_str() else {
        return Err(error(path, "Expected string"));
    };
    if nonempty && value.is_empty() {
        return Err(error(path, "Expected nonempty string"));
    }
    if value
        .chars()
        .any(|character| (character as u32) <= 0x1f || character == '\u{7f}')
    {
        return Err(error(path, "Control characters are not allowed"));
    }
    Ok(value)
}

fn boolean(value: &Value, path: &str) -> Result<bool, ManifestError> {
    value
        .as_bool()
        .ok_or_else(|| error(path, "Expected boolean"))
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a [Value], ManifestError> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| error(path, "Expected array"))
}

fn valid_version(value: &str) -> bool {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.into_iter().all(|part| {
        !part.is_empty()
            && !(part.len() > 1 && part.starts_with('0'))
            && part.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn valid_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn valid_token(value: &str, extra: &[u8]) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || extra.contains(&byte))
}

fn valid_repository(value: &str) -> bool {
    let Some((owner, repository)) = value.split_once('/') else {
        return false;
    };
    !repository.contains('/') && valid_token(owner, b"_.-") && valid_token(repository, b"_.-")
}

fn valid_formula(value: &str) -> bool {
    let parts: Vec<_> = value.split('/').collect();
    match parts.as_slice() {
        [formula] => valid_lower_token(formula, b"+_.@-"),
        [owner, tap, formula] => {
            valid_lower_token(owner, b"_.-")
                && valid_lower_token(tap, b"_.-")
                && valid_lower_token(formula, b"+_.@-")
        }
        _ => false,
    }
}

fn valid_lower_token(value: &str, extra: &[u8]) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || extra.contains(&byte))
}

fn valid_vcpkg_port(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn unique_strings(
    value: &Value,
    path: &str,
    valid: impl Fn(&str) -> bool,
) -> Result<Vec<String>, ManifestError> {
    let values = array(value, path)?;
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        let item = text(value, &item_path, true)?;
        if !valid(item) {
            return Err(error(&item_path, "Invalid identifier"));
        }
        if !seen.insert(item.to_owned()) {
            return Err(error(&item_path, "Duplicate entry"));
        }
        result.push(item.to_owned());
    }
    Ok(result)
}

fn validate_platforms(value: &Value, path: &str) -> Result<Vec<String>, ManifestError> {
    let values = unique_strings(value, path, |platform| {
        matches!(platform, "linux" | "macos" | "windows")
    })?;
    if values.is_empty() {
        return Err(error(path, "At least one platform is required"));
    }
    Ok(values)
}

fn validate_url(value: &Value, path: &str) -> Result<(), ManifestError> {
    let value = text(value, path, true)?;
    if !value.starts_with("https://")
        || value.chars().any(char::is_whitespace)
        || value.contains('#')
        || value.contains('\\')
    {
        return Err(error(
            path,
            "Expected HTTPS URL without credentials or fragment",
        ));
    }
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return Err(error(path, "Invalid HTTPS URL"));
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    let rest = &value[8..];
    let authority = rest.split(['/', '?']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return Err(error(
            path,
            "Expected HTTPS URL without credentials or fragment",
        ));
    }
    let port = if let Some(address) = authority.strip_prefix('[') {
        let Some((host, suffix)) = address.split_once(']') else {
            return Err(error(path, "Invalid HTTPS URL"));
        };
        if host.parse::<std::net::Ipv6Addr>().is_err() {
            return Err(error(path, "Invalid HTTPS URL"));
        }
        if suffix.is_empty() {
            None
        } else if let Some(port) = suffix.strip_prefix(':') {
            Some(port)
        } else {
            return Err(error(path, "Invalid HTTPS URL"));
        }
    } else {
        if authority.contains(['[', ']']) {
            return Err(error(path, "Invalid HTTPS URL"));
        }
        let mut parts = authority.split(':');
        let host = parts.next().unwrap_or_default();
        if host.is_empty() || parts.clone().count() > 1 {
            return Err(error(path, "Invalid HTTPS URL"));
        }
        parts.next()
    };
    if let Some(port) = port {
        let Ok(port) = port.parse::<u16>() else {
            return Err(error(path, "Invalid HTTPS URL"));
        };
        if port == 0 {
            return Err(error(path, "Invalid HTTPS URL"));
        }
    }
    Ok(())
}

fn validate_relative_path(value: &Value, path: &str, windows: bool) -> Result<(), ManifestError> {
    let value = text(value, path, true)?;
    if value
        .chars()
        .any(|character| matches!(character, '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*'))
    {
        return Err(error(
            path,
            "Expected relative path with forward-slash separators",
        ));
    }
    for component in value.split('/') {
        if component.is_empty() || matches!(component, "." | "..") {
            return Err(error(
                path,
                "Empty and traversal path components are not allowed",
            ));
        }
        if windows {
            let upper = component.to_ascii_uppercase();
            let stem = upper.split('.').next().unwrap_or_default();
            if component.ends_with([' ', '.'])
                || matches!(stem, "CON" | "PRN" | "AUX" | "NUL")
                || ["COM", "LPT"].iter().any(|prefix| {
                    stem.strip_prefix(prefix).is_some_and(|number| {
                        number.len() == 1
                            && number.as_bytes()[0].is_ascii_digit()
                            && number.as_bytes()[0] != b'0'
                    })
                })
            {
                return Err(error(path, "Path is not a valid Windows filename"));
            }
        }
    }
    Ok(())
}

fn validate_packages(value: &Value) -> Result<(), ManifestError> {
    let packages = object(value, "$.packages", &["linux", "macos", "windows"], &[])?;
    if let Some(linux) = packages.get("linux") {
        let linux = object(
            linux,
            "$.packages.linux",
            &["arch", "fedora", "ubuntu"],
            &[],
        )?;
        for (distro, fields) in [
            ("arch", &["official", "aur"][..]),
            ("fedora", &["copr", "packages"][..]),
            ("ubuntu", &["ppa", "packages"][..]),
        ] {
            let Some(branch) = linux.get(distro) else {
                continue;
            };
            let path = format!("$.packages.linux.{distro}");
            let branch = object(branch, &path, fields, &[])?;
            for (field, list) in branch {
                let item_path = format!("{path}.{field}");
                unique_strings(list, &item_path, |item| {
                    if field == "copr" || field == "ppa" {
                        valid_repository(item)
                    } else {
                        valid_token(item, b"+_.-")
                    }
                })?;
            }
        }
    }
    if let Some(macos) = packages.get("macos") {
        let macos = object(macos, "$.packages.macos", &["brew"], &[])?;
        if let Some(brew) = macos.get("brew") {
            let path = "$.packages.macos.brew";
            let brew = object(brew, path, &["taps", "formulae"], &[])?;
            if let Some(taps) = brew.get("taps") {
                unique_strings(taps, "$.packages.macos.brew.taps", valid_repository)?;
            }
            if let Some(formulae) = brew.get("formulae") {
                unique_strings(formulae, "$.packages.macos.brew.formulae", valid_formula)?;
            }
        }
    }
    if let Some(windows) = packages.get("windows") {
        validate_windows_packages(windows)?;
    }
    Ok(())
}

fn validate_windows_packages(value: &Value) -> Result<(), ManifestError> {
    let path = "$.packages.windows";
    let windows = object(value, path, &["winget_sources", "winget", "vcpkg"], &[])?;
    let mut source_names =
        std::collections::HashSet::from(["winget".to_owned(), "msstore".to_owned()]);
    if let Some(sources) = windows.get("winget_sources") {
        for (index, source) in array(sources, "$.packages.windows.winget_sources")?
            .iter()
            .enumerate()
        {
            let item_path = format!("$.packages.windows.winget_sources[{index}]");
            let source = object(
                source,
                &item_path,
                &["name", "url", "type"],
                &["name", "url", "type"],
            )?;
            let name = text(
                required(source, "name", &item_path)?,
                &format!("{item_path}.name"),
                true,
            )?;
            if !valid_token(name, b"_.-") {
                return Err(error(&format!("{item_path}.name"), "Invalid identifier"));
            }
            if !source_names.insert(name.to_ascii_lowercase()) {
                return Err(error(
                    &format!("{item_path}.name"),
                    "Duplicate or built-in source name",
                ));
            }
            validate_url(
                required(source, "url", &item_path)?,
                &format!("{item_path}.url"),
            )?;
            let kind = text(
                required(source, "type", &item_path)?,
                &format!("{item_path}.type"),
                true,
            )?;
            if !matches!(kind, "Microsoft.Rest" | "Microsoft.PreIndexed.Package") {
                return Err(error(
                    &format!("{item_path}.type"),
                    "Unsupported Windows source type",
                ));
            }
        }
    }
    if let Some(winget) = windows.get("winget") {
        let mut seen = std::collections::HashSet::new();
        for (index, item) in array(winget, "$.packages.windows.winget")?
            .iter()
            .enumerate()
        {
            let item_path = format!("$.packages.windows.winget[{index}]");
            let (identifier, source) = if let Some(identifier) = item.as_str() {
                if !valid_token(identifier, b"_.-") {
                    return Err(error(&item_path, "Invalid identifier"));
                }
                (identifier, "winget")
            } else {
                let item = object(item, &item_path, &["id", "source"], &["id", "source"])?;
                let identifier = text(
                    required(item, "id", &item_path)?,
                    &format!("{item_path}.id"),
                    true,
                )?;
                let source = text(
                    required(item, "source", &item_path)?,
                    &format!("{item_path}.source"),
                    true,
                )?;
                if !valid_token(identifier, b"_.-") {
                    return Err(error(&format!("{item_path}.id"), "Invalid identifier"));
                }
                if !valid_token(source, b"_.-") {
                    return Err(error(&format!("{item_path}.source"), "Invalid identifier"));
                }
                (identifier, source)
            };
            if !source_names.contains(&source.to_ascii_lowercase()) {
                return Err(error(
                    &item_path,
                    "Package references an undeclared Windows source",
                ));
            }
            let identity = format!("{identifier}\0{}", source.to_ascii_lowercase());
            if !seen.insert(identity) {
                return Err(error(&item_path, "Duplicate package requirement"));
            }
        }
    }
    if let Some(vcpkg) = windows.get("vcpkg") {
        unique_strings(vcpkg, "$.packages.windows.vcpkg", valid_vcpkg_port)?;
    }
    Ok(())
}

fn validate_requirement_list(
    values: Option<&Value>,
    field: &str,
    validator: fn(&Value, &str) -> Result<String, ManifestError>,
) -> Result<(), ManifestError> {
    let Some(values) = values else {
        return Ok(());
    };
    let path = format!("$.{field}");
    let mut seen = std::collections::HashSet::new();
    for (index, value) in array(values, &path)?.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        let id = validator(value, &item_path)?;
        if !seen.insert(id) {
            return Err(error(
                &format!("{item_path}.id"),
                "Duplicate requirement ID",
            ));
        }
    }
    Ok(())
}

fn validate_manual(value: &Value, path: &str) -> Result<String, ManifestError> {
    let fields = [
        "id",
        "platforms",
        "url",
        "description",
        "verify",
        "environment",
    ];
    let required_fields = ["id", "platforms", "url", "description", "verify"];
    let manual = object(value, path, &fields, &required_fields)?;
    let id = text(required(manual, "id", path)?, &format!("{path}.id"), true)?;
    if !valid_identifier(id) {
        return Err(error(&format!("{path}.id"), "Invalid identifier"));
    }
    let platforms = validate_platforms(
        required(manual, "platforms", path)?,
        &format!("{path}.platforms"),
    )?;
    validate_url(required(manual, "url", path)?, &format!("{path}.url"))?;
    text(
        required(manual, "description", path)?,
        &format!("{path}.description"),
        true,
    )?;
    let verify_path = format!("{path}.verify");
    let verify = object(
        required(manual, "verify", path)?,
        &verify_path,
        &["path"],
        &["path"],
    )?;
    validate_relative_path(
        required(verify, "path", &verify_path)?,
        &format!("{verify_path}.path"),
        platforms.iter().any(|p| p == "windows"),
    )?;
    if let Some(environment) = manual.get("environment") {
        let environment_path = format!("{path}.environment");
        let environment = environment
            .as_object()
            .ok_or_else(|| error(&environment_path, "Expected object"))?;
        let mut names = std::collections::HashSet::new();
        for (name, value) in environment {
            let value_path = format!("{environment_path}.{name}");
            let mut bytes = name.bytes();
            let name_valid = bytes
                .next()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
            if !name_valid {
                return Err(error(&value_path, "Invalid identifier"));
            }
            let identity = if platforms.iter().any(|p| p == "windows") {
                name.to_ascii_uppercase()
            } else {
                name.clone()
            };
            if !names.insert(identity) {
                return Err(error(&value_path, "Conflicting environment variable names"));
            }
            let (environment_value, operation, scope) = if value.is_string() {
                (text(value, &value_path, true)?, "set", "user")
            } else {
                let settings = object(
                    value,
                    &value_path,
                    &["value", "operation", "scope"],
                    &["value", "operation", "scope"],
                )?;
                let environment_value = text(
                    required(settings, "value", &value_path)?,
                    &format!("{value_path}.value"),
                    true,
                )?;
                let operation = text(
                    required(settings, "operation", &value_path)?,
                    &format!("{value_path}.operation"),
                    true,
                )?;
                let scope = text(
                    required(settings, "scope", &value_path)?,
                    &format!("{value_path}.scope"),
                    true,
                )?;
                (environment_value, operation, scope)
            };
            if !valid_environment_value(environment_value) {
                return Err(error(
                    &format!("{value_path}.value"),
                    "Unsupported placeholder",
                ));
            }
            if !matches!(operation, "set" | "append_path") || !matches!(scope, "project" | "user") {
                return Err(error(
                    &value_path,
                    "Unsupported environment operation or scope",
                ));
            }
        }
    }
    Ok(id.to_owned())
}

fn valid_environment_value(value: &str) -> bool {
    let mut remaining = value;
    while let Some(open) = remaining.find(['{', '}']) {
        if remaining.as_bytes()[open] == b'}' {
            return false;
        }
        let Some(after) = remaining[open..].strip_prefix("{selected_install_directory}") else {
            return false;
        };
        remaining = after;
    }
    true
}

fn validate_asset(value: &Value, path: &str) -> Result<String, ManifestError> {
    const REQUIRED: &[&str] = &[
        "id",
        "url",
        "sha256",
        "size_bytes",
        "license_url",
        "destination",
        "description",
        "archive_format",
        "max_extracted_size_bytes",
        "max_files",
    ];
    let fields = [
        "id",
        "url",
        "sha256",
        "size_bytes",
        "license_url",
        "destination",
        "description",
        "archive_format",
        "max_extracted_size_bytes",
        "max_files",
        "platforms",
        "destination_root",
        "optional",
    ];
    let asset = object(value, path, &fields, REQUIRED)?;
    let id = text(required(asset, "id", path)?, &format!("{path}.id"), true)?;
    if !valid_identifier(id) {
        return Err(error(&format!("{path}.id"), "Invalid identifier"));
    }
    let platforms = if let Some(platforms) = asset.get("platforms") {
        validate_platforms(platforms, &format!("{path}.platforms"))?
    } else {
        vec!["linux".to_owned(), "macos".to_owned(), "windows".to_owned()]
    };
    validate_url(required(asset, "url", path)?, &format!("{path}.url"))?;
    validate_url(
        required(asset, "license_url", path)?,
        &format!("{path}.license_url"),
    )?;
    let checksum = text(
        required(asset, "sha256", path)?,
        &format!("{path}.sha256"),
        true,
    )?;
    if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(error(&format!("{path}.sha256"), "Invalid identifier"));
    }
    for field in ["size_bytes", "max_extracted_size_bytes", "max_files"] {
        let number = required(asset, field, path)?.as_u64();
        if !number.is_some_and(|number| (1..=9_007_199_254_740_991).contains(&number)) {
            return Err(error(
                &format!("{path}.{field}"),
                "Integer must be between 1 and 9007199254740991",
            ));
        }
    }
    validate_relative_path(
        required(asset, "destination", path)?,
        &format!("{path}.destination"),
        platforms.iter().any(|p| p == "windows"),
    )?;
    text(
        required(asset, "description", path)?,
        &format!("{path}.description"),
        true,
    )?;
    if let Some(root) = asset.get("destination_root") {
        let root = text(root, &format!("{path}.destination_root"), true)?;
        if !matches!(root, "project" | "module") {
            return Err(error(
                &format!("{path}.destination_root"),
                "Unsupported destination root",
            ));
        }
    }
    let archive_format = text(
        required(asset, "archive_format", path)?,
        &format!("{path}.archive_format"),
        true,
    )?;
    if archive_format != "zip" {
        return Err(error(
            &format!("{path}.archive_format"),
            "Unsupported archive format",
        ));
    }
    if let Some(optional) = asset.get("optional") {
        boolean(optional, &format!("{path}.optional"))?;
    }
    Ok(id.to_owned())
}

fn insert_defaults(data: &mut Value) {
    let object = data.as_object_mut().expect("validated top-level object");
    object.entry("schema_version").or_insert(Value::from(1));
    object.entry("needs_lila").or_insert(Value::Bool(false));
    object
        .entry("packages")
        .or_insert_with(|| Value::Object(Default::default()));
    object
        .entry("manual_installs")
        .or_insert_with(|| Value::Array(Vec::new()));
    object
        .entry("assets")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Some(manuals) = object
        .get_mut("manual_installs")
        .and_then(Value::as_array_mut)
    {
        for manual in manuals {
            let Some(manual) = manual.as_object_mut() else {
                continue;
            };
            let environment = manual
                .entry("environment")
                .or_insert_with(|| Value::Object(Default::default()));
            if let Some(environment) = environment.as_object_mut() {
                for change in environment.values_mut() {
                    if let Some(value) = change.as_str() {
                        *change = serde_json::json!({ "value": value, "operation": "set", "scope": "user" });
                    }
                }
            }
        }
    }
    if let Some(assets) = object.get_mut("assets").and_then(Value::as_array_mut) {
        for asset in assets {
            if let Some(asset) = asset.as_object_mut() {
                asset
                    .entry("platforms")
                    .or_insert_with(|| serde_json::json!(["linux", "macos", "windows"]));
                asset
                    .entry("destination_root")
                    .or_insert_with(|| Value::String("project".to_owned()));
                asset.entry("optional").or_insert(Value::Bool(false));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_manifest;
    use serde_json::Value;
    use std::{fs, path::PathBuf};

    fn fixture_directory() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/community")
    }

    #[test]
    fn every_legacy_vector_is_accepted_or_rejected_as_named() {
        let mut paths: Vec<_> = fs::read_dir(fixture_directory())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect();
        paths.sort();
        assert!(paths.iter().any(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("valid-")
        }));
        assert!(paths.iter().any(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("invalid-")
        }));
        for path in paths {
            let source = fs::read_to_string(&path).unwrap();
            let value: Value = serde_json::from_str(&source).unwrap();
            let name = value["name"].as_str().unwrap();
            let result = parse_manifest(&source, name);
            let file_name = path.file_name().unwrap().to_string_lossy();
            if file_name.starts_with("valid-") {
                assert!(result.is_ok(), "{}: {result:?}", path.display());
            } else {
                assert!(file_name.starts_with("invalid-"), "{file_name}");
                assert!(result.is_err(), "{} was accepted", path.display());
            }
        }
    }

    #[test]
    fn metadata_only_legacy_manifest_receives_compatible_defaults() {
        let manifest = parse_manifest(
            r#"{"name":"sample_module","min_version":"0.5.0","description":"legacy"}"#,
            "sample_module",
        )
        .unwrap();

        assert_eq!(manifest.minimum_version, "0.5.0");
        assert_eq!(manifest.data["schema_version"], 1);
        assert_eq!(manifest.data["needs_lila"], false);
        assert_eq!(manifest.data["packages"], serde_json::json!({}));
        assert_eq!(manifest.data["manual_installs"], serde_json::json!([]));
        assert_eq!(manifest.data["assets"], serde_json::json!([]));
        assert_eq!(manifest.data["description"], "legacy");
    }

    #[test]
    fn acquisition_declarations_require_revision_one() {
        let source = r#"{"name":"sample_module","min_version":"0.5.0","packages":{}}"#;
        let error = parse_manifest(source, "sample_module").unwrap_err();
        assert_eq!(error.path, "$.schema_version");
        assert!(error.reason.contains("require schema revision 1"));
    }

    #[test]
    fn parser_rejects_duplicate_keys_noninteger_tokens_and_excessive_nesting() {
        for source in [
            r#"{"name":"sample_module","name":"sample_module","min_version":"0.5.0"}"#,
            r#"{"schema_version":1.0,"name":"sample_module","min_version":"0.5.0"}"#,
            r#"{"schema_version":1e0,"name":"sample_module","min_version":"0.5.0"}"#,
        ] {
            assert!(parse_manifest(source, "sample_module").is_err(), "{source}");
        }

        let nested = format!(
            "{{\"name\":\"sample_module\",\"min_version\":\"0.5.0\",\"description\":{}}}",
            format!("{}null{}", "[".repeat(64), "]".repeat(64))
        );
        assert!(parse_manifest(&nested, "sample_module").is_err());

        let asset_template = r#"{"schema_version":1,"name":"sample_module","min_version":"0.5.0","assets":[{"id":"data","url":"https://example.org/data.zip","sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","size_bytes":SIZE,"license_url":"https://example.org/license","destination":"assets/data","description":"Data archive","archive_format":"zip","max_extracted_size_bytes":10,"max_files":1}]}"#;
        for token in ["1.0", "1e0"] {
            let source = asset_template.replace("SIZE", token);
            assert!(parse_manifest(&source, "sample_module").is_err(), "{token}");
        }

        let duplicate = parse_manifest(
            r#"{"name":"sample_module","name":"sample_module","min_version":"0.5.0"}"#,
            "sample_module",
        )
        .unwrap_err();
        assert_eq!(duplicate.path, "$.name");
        assert!(duplicate.reason.contains("Duplicate object key"));
    }

    #[test]
    fn parser_accepts_bom_and_rejects_requested_name_mismatch() {
        let source = "\u{feff}{\"name\":\"sample_module\",\"min_version\":\"0.5.0\"}";
        assert!(parse_manifest(source, "sample_module").is_ok());
        assert!(parse_manifest(source, "other_module").is_err());
    }

    #[test]
    fn parser_preserves_legacy_manual_and_asset_defaults() {
        let manual_source =
            fs::read_to_string(fixture_directory().join("valid-manual-install-environment.json"))
                .unwrap();
        let manual = parse_manifest(&manual_source, "vector_module").unwrap();
        assert_eq!(
            manual.data["manual_installs"][0]["environment"]["VENDOR_SDK_ROOT"],
            serde_json::json!({
                "value": "{selected_install_directory}",
                "operation": "set",
                "scope": "user"
            })
        );

        let asset_source =
            fs::read_to_string(fixture_directory().join("valid-asset-zip-limits.json")).unwrap();
        let asset = parse_manifest(&asset_source, "vector_module").unwrap();
        assert_eq!(
            asset.data["assets"][0]["platforms"],
            serde_json::json!(["linux", "macos", "windows"])
        );
        assert_eq!(asset.data["assets"][0]["destination_root"], "project");
        assert_eq!(asset.data["assets"][0]["optional"], false);
    }

    #[test]
    fn parser_accepts_unbounded_decimal_minimum_version_components() {
        let source = r#"{"name":"sample_module","min_version":"999999999999999999999999.0.0"}"#;
        assert_eq!(
            parse_manifest(source, "sample_module")
                .unwrap()
                .minimum_version,
            "999999999999999999999999.0.0"
        );
    }

    #[test]
    fn urls_require_https_host_valid_port_and_well_formed_escapes() {
        for url in [
            "http://example.org/file",
            "https:///file",
            "https://user:password@example.org/file",
            "https://example.org:65536/file",
            "https://example.org/%xx",
        ] {
            let source = format!(
                r#"{{"schema_version":1,"name":"sample_module","min_version":"0.5.0","manual_installs":[{{"id":"sdk","platforms":["linux"],"url":"{url}","description":"SDK","verify":{{"path":"bin/sdk"}}}}]}}"#
            );
            assert!(parse_manifest(&source, "sample_module").is_err(), "{url}");
        }
    }
}
