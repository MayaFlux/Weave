//! Shared environment-write recovery and path-list merging.
use std::{collections::BTreeMap, path::PathBuf};
pub trait Store {
    fn read(&self, name: &str) -> Result<String, String>;
    fn write(&mut self, name: &str, value: &str) -> Result<(), String>;
}

pub fn apply(values: &BTreeMap<String, String>, store: &mut impl Store) -> Result<(), String> {
    for (name, value) in values {
        let existing = store
            .read(name)
            .map_err(|e| format!("Cannot read {name}: {e}; retry environment setup"))?;
        if existing != *value {
            store.write(name, value).map_err(|e| format!("Cannot update {name}: {e}. Earlier environment changes may already be applied; correct permissions and retry environment setup"))?;
        }
    }
    Ok(())
}

pub fn merge_paths(
    existing: &str,
    additions: &[PathBuf],
    prefer: bool,
    separator: char,
    normalize: impl Fn(&str) -> String,
) -> Result<String, String> {
    for path in additions {
        crate::filesystem::safe_directory(path)?;
    }
    let new: Vec<_> = additions
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let old: Vec<_> = existing
        .split(separator)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let ordered = if prefer {
        new.into_iter().chain(old).collect::<Vec<_>>()
    } else {
        old.into_iter().chain(new).collect()
    };
    let mut entries = Vec::<String>::new();
    for entry in ordered {
        if !entries
            .iter()
            .any(|old| normalize(old) == normalize(&entry))
        {
            entries.push(entry);
        }
    }
    Ok(entries.join(&separator.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Fake {
        values: BTreeMap<String, String>,
        fail: Option<String>,
    }
    impl Store for Fake {
        fn read(&self, name: &str) -> Result<String, String> {
            Ok(self.values.get(name).cloned().unwrap_or_default())
        }
        fn write(&mut self, name: &str, value: &str) -> Result<(), String> {
            if self.fail.as_deref() == Some(name) {
                return Err("Access denied".into());
            }
            self.values.insert(name.into(), value.into());
            Ok(())
        }
    }
    #[test]
    fn failed_machine_write_reports_partial_state_and_retry_is_idempotent() {
        let values = BTreeMap::from([
            ("MAYAFLUX_ROOT".into(), "D:\\Maya Flux".into()),
            ("PATH".into(), "D:\\Tools".into()),
        ]);
        let mut store = Fake {
            fail: Some("PATH".into()),
            ..Default::default()
        };
        let error = apply(&values, &mut store).unwrap_err();
        assert!(error.contains("PATH"));
        assert!(error.contains("retry"));
        assert_eq!(
            store.values.get("MAYAFLUX_ROOT").map(String::as_str),
            Some("D:\\Maya Flux")
        );
        store.fail = None;
        apply(&values, &mut store).unwrap();
        apply(&values, &mut store).unwrap();
        assert_eq!(store.values, values);
        store.fail = Some("PATH".into());
        let changed = BTreeMap::from([("PATH".into(), "D:\\New Tools".into())]);
        assert!(apply(&changed, &mut store).is_err());
    }
}
