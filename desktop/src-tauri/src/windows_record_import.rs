//! Windows F3 path/identity adapter. Pins are retained through the transaction.
use sha2::{Digest, Sha256};
use std::path::{Component, Path};
use xarchive_workflow::windows_filesystem::PinnedPath;

pub(crate) fn update_root_identity(root: &Path, digest: &mut Sha256) -> Result<(), String> {
    let pin = PinnedPath::directory(root, false).map_err(|e| e.to_string())?;
    let identity = fs_id::FileID::new(pin.file()).map_err(|e| e.to_string())?;
    update_identity(digest, identity.storage_id(), identity.internal_file_id());
    Ok(())
}

fn update_identity(digest: &mut Sha256, volume: u64, file: u128) {
    // ReFS uses 128-bit file IDs. Hash the full FILE_ID_INFO from the already
    // pinned handle, with an explicit format independent of Rust's Hash trait.
    digest.update(b"windows-file-id-v2\0");
    digest.update(volume.to_le_bytes());
    digest.update(file.to_le_bytes());
}

pub(crate) struct EvidencePins {
    root: std::path::PathBuf,
    _pins: Vec<PinnedPath>,
}
impl EvidencePins {
    pub(crate) fn new(root: &Path) -> Result<Self, String> {
        Ok(Self {
            root: root.to_owned(),
            _pins: vec![PinnedPath::directory(root, false).map_err(|e| e.to_string())?],
        })
    }
    pub(crate) fn add(&mut self, relative: &str, directory: bool) -> Result<(), String> {
        let relative = Path::new(relative);
        if relative.is_absolute()
            || relative.components().any(|p| {
                matches!(
                    p,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err("record evidence escapes root".into());
        }
        let mut path = self.root.clone();
        for part in relative.components() {
            if let Component::Normal(name) = part {
                path.push(name);
            }
        }
        let pin = if directory {
            PinnedPath::directory(&path, false)
        } else {
            PinnedPath::media(&path)
        };
        self._pins
            .push(pin.map_err(|e| format!("record evidence pin: {e}"))?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    fn fixture_base() -> std::path::PathBuf {
        let base = std::env::var_os("TW2TG_RECORD_ID_FIXTURE_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::canonicalize(base).unwrap()
    }
    fn root(name: &str) -> std::path::PathBuf {
        let base = fixture_base();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = base.join(format!(
            "tw2tg-win-import-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        root
    }
    fn remove_fixture(root: &Path) {
        let base = fixture_base();
        let resolved = fs::canonicalize(root).unwrap();
        assert_eq!(resolved.parent(), Some(base.as_path()));
        assert!(
            resolved
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("tw2tg-win-import-")
        );
        fs::remove_dir_all(resolved).unwrap();
    }
    fn revision(root: &Path) -> String {
        let mut digest = Sha256::new();
        update_root_identity(root, &mut digest).unwrap();
        format!("{:x}", digest.finalize())
    }
    #[test]
    fn revision_uses_the_entire_windows_file_identity() {
        fn hash(volume: u64, file: u128) -> [u8; 32] {
            let mut digest = Sha256::new();
            update_identity(&mut digest, volume, file);
            digest.finalize().into()
        }
        assert_ne!(hash(1, 1), hash(1, (1u128 << 64) | 1));
        assert_ne!(hash(1, 1), hash(2, 1));
    }
    #[test]
    fn root_identity_detects_replacement_at_the_same_path() {
        let root = root("identity");
        let old = revision(&root);
        fs::rename(&root, root.with_extension("old")).unwrap();
        fs::create_dir(&root).unwrap();
        assert_ne!(old, revision(&root));
        remove_fixture(&root);
        remove_fixture(&root.with_extension("old"));
    }
    #[test]
    fn unicode_media_and_dot_archive_directory_stay_pinned() {
        let root = root("空 格");
        fs::write(root.join("媒体.bin"), b"content").unwrap();
        let mut pins = EvidencePins::new(&root).unwrap();
        pins.add(".", true).unwrap();
        pins.add("媒体.bin", false).unwrap();
        assert!(fs::write(root.join("媒体.bin"), b"changed").is_err());
        assert!(pins.add("../outside", false).is_err());
        assert!(pins.add("媒体.bin:alternate", false).is_err());
        drop(pins);
        remove_fixture(&root);
    }
}
