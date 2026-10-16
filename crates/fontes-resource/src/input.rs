use crate::Report;
use anyhow::{Context, Result};
use std::{
    collections::{BTreeSet, HashSet},
    fs,
    io::{Read, Write},
    path::Path,
};

pub(crate) const MAX_XML: u64 = 16 * 1024 * 1024;
const MAX_TOTAL: u64 = 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 10_000;

pub(crate) struct Snapshot {
    pub root: tempfile::TempDir,
    pub files: BTreeSet<String>,
}

pub(crate) fn safe_path(path: &str) -> bool {
    path.len() <= 240
        && !path.is_empty()
        && path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        })
}

struct Loader<'a> {
    snapshot: Snapshot,
    seen: HashSet<String>,
    total: u64,
    count: usize,
    report: &'a mut Report,
}

impl Snapshot {
    pub fn load(path: &Path, report: &mut Report) -> Result<Option<Self>> {
        let mut loader = Loader {
            snapshot: Self {
                root: tempfile::tempdir()?,
                files: BTreeSet::new(),
            },
            seen: HashSet::new(),
            total: 0,
            count: 0,
            report,
        };
        let metadata = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(error) => {
                loader.report.global(
                    "INPUT_READ",
                    &path.display().to_string(),
                    error.to_string(),
                    "Provide an existing source directory or source ZIP.",
                );
                return Ok(None);
            }
        };
        if metadata.file_type().is_symlink() {
            loader.report.global(
                "INPUT_LINK",
                &path.display().to_string(),
                "Source is a symbolic link.",
                "Use the real source directory or ZIP path.",
            );
            return Ok(None);
        }
        if metadata.is_dir() {
            loader.directory(path, "", 0)?;
        } else if metadata.is_file() {
            if let Err(error) = loader.archive(path) {
                loader.report.global("ARCHIVE_READ", &path.display().to_string(), error.to_string(), "Use an unencrypted, undamaged ZIP with supported compression and safe regular files.");
            }
        } else {
            loader.report.global(
                "INPUT_TYPE",
                &path.display().to_string(),
                "Source is not a regular file or directory.",
                "Use a source directory or ZIP.",
            );
        }
        Ok(Some(loader.snapshot))
    }
}

impl Loader<'_> {
    fn name(&mut self, name: &str) -> bool {
        self.count += 1;
        if self.count > MAX_ENTRIES {
            self.report.global(
                "ENTRY_LIMIT",
                name,
                "More than 10,000 source entries.",
                "Remove extra files or split the resource.",
            );
            return false;
        }
        if !safe_path(name) || name.split('/').count() > 32 {
            self.report.global("UNSAFE_PATH", name, "Unsafe, unsupported, or excessively nested source path.", "Use relative portable ASCII paths; remove traversal, backslashes, and deep nesting.");
            return false;
        }
        if !self.seen.insert(name.to_ascii_lowercase()) {
            self.report.global(
                "DUPLICATE_PATH",
                name,
                "Duplicate path or case-insensitive name collision.",
                "Give each file/directory one distinct name, including when compared without case.",
            );
            return false;
        }
        true
    }

    fn directory(&mut self, root: &Path, prefix: &str, depth: usize) -> Result<()> {
        if depth > 32 {
            return Ok(());
        }
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let Some(component) = entry.file_name().to_str().map(str::to_owned) else {
                self.report.global(
                    "UNSAFE_PATH",
                    prefix,
                    "Path is not UTF-8.",
                    "Rename files using the portable ASCII path subset.",
                );
                continue;
            };
            let name = format!("{prefix}{component}");
            if !self.name(&name) {
                if self.count > MAX_ENTRIES {
                    break;
                }
                continue;
            }
            let meta = fs::symlink_metadata(entry.path())?;
            if meta.is_dir() {
                self.directory(&entry.path(), &format!("{name}/"), depth + 1)?;
            } else if meta.is_file() {
                self.copy(&name, fs::File::open(entry.path())?)?;
            } else {
                self.report.global(
                    "FILE_TYPE",
                    &name,
                    "Links and special files are forbidden.",
                    "Replace links with actual regular source files.",
                );
            }
            if self.total > MAX_TOTAL {
                break;
            }
        }
        Ok(())
    }

    fn archive(&mut self, path: &Path) -> Result<()> {
        let mut archive = zip::ZipArchive::new(fs::File::open(path)?)?;
        if archive.len() > MAX_ENTRIES {
            self.report.global(
                "ENTRY_LIMIT",
                "",
                "ZIP has more than 10,000 entries.",
                "Remove extra entries or split the resource.",
            );
            return Ok(());
        }
        for index in 0..archive.len() {
            let file = archive.by_index(index).context("cannot read ZIP entry")?;
            let name = std::str::from_utf8(file.name_raw())?.to_owned();
            let is_dir = file.is_dir();
            let name = if is_dir {
                name.strip_suffix('/').unwrap_or(&name).to_owned()
            } else {
                name
            };
            if !self.name(&name) {
                continue;
            }
            let mode = file
                .unix_mode()
                .unwrap_or(if is_dir { 0o040755 } else { 0o100644 })
                & 0o170000;
            if file.encrypted()
                || file.is_symlink()
                || !matches!(mode, 0 | 0o040000 | 0o100000)
                || (mode == 0o040000 && !is_dir)
            {
                self.report.global(
                    "ARCHIVE_ENTRY",
                    &name,
                    "Encrypted, linked, or special ZIP entry.",
                    "Repackage regular unencrypted files only.",
                );
                continue;
            }
            if !is_dir {
                self.copy(&name, file)?;
            }
            if self.total > MAX_TOTAL {
                break;
            }
        }
        Ok(())
    }

    fn copy(&mut self, name: &str, reader: impl Read) -> Result<()> {
        let remaining = MAX_TOTAL.saturating_sub(self.total);
        let limit = remaining.min(if name.ends_with(".xml") {
            MAX_XML
        } else {
            MAX_TOTAL
        });
        let target = self.snapshot.root.path().join(name);
        fs::create_dir_all(target.parent().context("source path parent")?)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        let actual = std::io::copy(&mut reader.take(limit + 1), &mut file)?;
        file.flush()?;
        self.total += actual;
        if actual > limit {
            self.report.global("SIZE_LIMIT", name, "Actual expanded bytes exceed the XML or total source limit.", "Keep XML below 16 MiB and total source below 1 GiB; split oversized content files.");
            return Ok(());
        }
        self.snapshot.files.insert(name.into());
        Ok(())
    }
}
