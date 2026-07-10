use std::{
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

use hls_core::{HlsError, HlsResult};

const MAX_RUN_ID_BYTES: usize = 128;

pub fn validate_run_id(run_id: &str) -> HlsResult<()> {
    if run_id.is_empty() {
        return Err(HlsError::Config("run ID must not be empty".to_owned()));
    }
    if run_id.len() > MAX_RUN_ID_BYTES {
        return Err(HlsError::Config(format!(
            "run ID must be at most {MAX_RUN_ID_BYTES} ASCII bytes"
        )));
    }
    if matches!(run_id, "." | "..")
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(HlsError::Config(
            "run ID may contain only ASCII letters, digits, '.', '-', and '_' and must not contain path components"
                .to_owned(),
        ));
    }
    Ok(())
}

pub fn validate_registered_data_path(path: &str) -> HlsResult<()> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(HlsError::Config(format!(
            "registered data path '{}' must be a non-empty relative path without parent components",
            path.display()
        )));
    }
    Ok(())
}

pub fn resolve_registered_data_path(data_dir: &Path, path: &str) -> HlsResult<PathBuf> {
    validate_registered_data_path(path)?;
    let canonical_data_dir = fs::canonicalize(data_dir)?;
    let resolved = fs::canonicalize(data_dir.join(path))?;
    if !resolved.starts_with(&canonical_data_dir) {
        return Err(HlsError::Config(format!(
            "registered data path '{}' resolves outside data directory '{}'",
            path,
            data_dir.display()
        )));
    }
    Ok(resolved)
}

pub fn prepare_data_file_path(data_dir: &Path, path: &str) -> HlsResult<PathBuf> {
    validate_registered_data_path(path)?;
    fs::create_dir_all(data_dir)?;
    let canonical_data_dir = fs::canonicalize(data_dir)?;
    let relative = Path::new(path);
    let mut current = canonical_data_dir.clone();
    if let Some(parent) = relative.parent() {
        for component in parent.components() {
            let Component::Normal(component) = component else {
                return Err(HlsError::Config(format!(
                    "registered data path '{}' contains an invalid component",
                    relative.display()
                )));
            };
            current.push(component);
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(HlsError::Config(format!(
                        "refusing symbolic link in data path '{}'",
                        current.display()
                    )));
                }
                Ok(metadata) if !metadata.is_dir() => {
                    return Err(HlsError::Config(format!(
                        "data path parent '{}' is not a directory",
                        current.display()
                    )));
                }
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    fs::create_dir(&current)?;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    let full_path = canonical_data_dir.join(relative);
    if fs::symlink_metadata(&full_path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(HlsError::Config(format!(
            "refusing symbolic link at data file '{}'",
            full_path.display()
        )));
    }
    Ok(full_path)
}
