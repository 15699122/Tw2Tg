#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use xarchive_sidecar_supervisor::{ChildProcessEnvironment, SidecarSupervisor};
    use xarchive_workflow::coordination::windows::WindowsCoordinator;
    use xarchive_workflow::windows_containment::{WindowsWriterIdentity, WindowsWriterJob};
    use xarchive_workflow::writer_lifecycle::{PublicationError, PublicationOwner};
    use xarchive_workflow::{AttemptIdentity, ExecutionReservation};

    if std::env::args_os()
        .skip(1)
        .eq([std::ffi::OsStr::new("--xarchive-writer-keeper")])
    {
        loop {
            std::thread::park();
        }
    }
    let python = PathBuf::from(std::env::var_os("TW2TG_PYTHON").ok_or("TW2TG_PYTHON required")?);
    let root = std::env::temp_dir().join(format!("tw2tg-supervisor-native-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    let coordinator = WindowsCoordinator::new(&root)?;
    let identity = AttemptIdentity::new("supervisor-native", 1)?;
    let session =
        ExecutionReservation::acquire(&coordinator, "supervisor-native", &root.join("archive"))?
            .bind_existing_attempt(identity.clone())?;
    let publication = PublicationOwner::for_session(&session);
    let writer = WindowsWriterIdentity::new(&root, &identity, "sidecar")?;
    let mut job = WindowsWriterJob::create_recorded(&root, writer)?;
    publication.registration().register_windows(&mut job)?;
    let script = r#"import json, sys, os
assert os.environ['TW2TG_PIPE_SET'] == 'expected'
assert 'TW2TG_PIPE_REMOVED' not in os.environ
assert sys.argv[1:] == ['with spaces\\', 'quote"value', ''], repr(sys.argv[1:])
for line in sys.stdin:
 c=json.loads(line)
 if c['cmd']=='hello': print(json.dumps({'protocol_version':2,'event':'ready','job_id':'system','request_id':c['request_id'],'capabilities':['extract_media','download_media','cancel_active_extraction','structured_media_plan','account_discovery']}),flush=True)
 elif c['cmd']=='shutdown': break
"#;
    let mut supervisor = SidecarSupervisor::spawn_windows_contained_with_environment(
        job,
        Path::new(&python),
        &["-c", script, "with spaces\\", "quote\"value", ""],
        &ChildProcessEnvironment::new(
            vec![
                ("TW2TG_PIPE_SET".into(), "expected".into()),
                ("TW2TG_PIPE_REMOVED".into(), "remove-me".into()),
            ],
            vec!["tw2tg_pipe_removed".into()],
        ),
    )?
    .require_ready_v2(Duration::from_secs(5))?;
    assert!(supervisor.is_running()?);
    publication
        .close_admission()
        .expect("close writer admission");
    assert_eq!(
        publication.authorize(&session).err(),
        Some(PublicationError::IncompleteDrain)
    );
    supervisor.send_v2_shutdown("shutdown")?;
    supervisor.close_stdin();
    assert!(supervisor.wait_for_exit(Duration::from_secs(5))?);
    assert_eq!(
        publication.authorize(&session).err(),
        Some(PublicationError::IncompleteDrain)
    );
    drop((supervisor, publication));
    drop(session);
    drop(coordinator);
    std::fs::remove_dir_all(root)?;
    println!("windows contained supervisor: PASS");
    Ok(())
}
