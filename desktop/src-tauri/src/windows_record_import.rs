//! Windows F3 path/identity adapter. Pins are retained through the transaction.
use sha2::{Digest, Sha256};
use std::hash::{Hash, Hasher};
use std::path::{Component, Path};
use xarchive_workflow::windows_filesystem::PinnedPath;

pub(crate) fn update_root_identity(root: &Path, digest: &mut Sha256) -> Result<(), String> {
    let pin = PinnedPath::directory(root, false).map_err(|e| e.to_string())?;
    let handle = same_file::Handle::from_file(pin.file().try_clone().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    // Stream the Windows volume/file identity into the same SHA-256 revision;
    // do not substitute a timestamp or a pathname for object identity.
    struct IdentityDigest<'a>(&'a mut Sha256);
    impl Hasher for IdentityDigest<'_> {
        fn write(&mut self, bytes: &[u8]) {
            self.0.update(bytes);
        }
        fn finish(&self) -> u64 {
            0
        }
    }
    handle.hash(&mut IdentityDigest(digest));
    Ok(())
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
    fn root(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("tw2tg-win-import-{name}-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }
    fn revision(root: &Path) -> String {
        let mut digest = Sha256::new();
        update_root_identity(root, &mut digest).unwrap();
        format!("{:x}", digest.finalize())
    }
    #[test]
    fn root_identity_detects_replacement_at_the_same_path() {
        let root = root("identity");
        let old = revision(&root);
        fs::rename(&root, root.with_extension("old")).unwrap();
        fs::create_dir(&root).unwrap();
        assert_ne!(old, revision(&root));
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(root.with_extension("old")).unwrap();
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
        fs::remove_dir_all(root).unwrap();
    }
}
