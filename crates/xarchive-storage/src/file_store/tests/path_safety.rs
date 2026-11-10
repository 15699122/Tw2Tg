use super::*;

#[test]
fn rejects_parent_and_absolute_paths() {
    let root = temp_root("lexical");
    let store = FileStore::new(&root).expect("file store");

    assert!(matches!(
        store.write_text(Path::new("../escape.txt"), "x"),
        Err(StorageError::InvalidPath)
    ));
    assert!(matches!(
        store.write_text(Path::new("/absolute.txt"), "x"),
        Err(StorageError::InvalidPath)
    ));
    let _ = fs::remove_dir_all(&root);
}

/// ENG-03: an intermediate directory that is a symlink (or, on Windows, a
/// reparse point / junction) must not allow writes outside the root.
#[cfg(unix)]
#[test]
fn rejects_an_intermediate_symlink_that_points_outside_the_root() {
    use std::os::unix::fs::symlink;

    let root = temp_root("symlink");
    let outside = temp_root("symlink-outside");
    fs::write(outside.join("secret.txt"), "sensitive").expect("outside file");

    let store = FileStore::new(&root).expect("file store");
    symlink(&outside, root.join("escape")).expect("create symlink");

    let result = store.write_text(Path::new("escape/payload.txt"), "x");
    assert!(
        matches!(result, Err(StorageError::InvalidPath)),
        "intermediate symlink must be rejected, got {result:?}"
    );
    assert!(
        !outside.join("payload.txt").exists(),
        "write escaped the archive root"
    );

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[test]
fn rejects_a_final_symlink_component() {
    use std::os::unix::fs::symlink;

    let root = temp_root("final-symlink");
    let outside = temp_root("final-symlink-outside");
    fs::write(outside.join("target.txt"), "sensitive").expect("outside file");

    let store = FileStore::new(&root).expect("file store");
    symlink(outside.join("target.txt"), root.join("link.txt")).expect("symlink");

    let result = store.write_text(Path::new("link.txt"), "overwritten");
    assert!(
        matches!(result, Err(StorageError::InvalidPath)),
        "final symlink must be rejected, got {result:?}"
    );
    assert_eq!(
        fs::read_to_string(outside.join("target.txt")).expect("read"),
        "sensitive"
    );

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(outside);
}
