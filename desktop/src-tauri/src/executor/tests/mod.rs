//! Executor regression suites.
//!
//! Split out of `executor.rs` without renaming the module, so the module path
//! stays `executor::tests::*` and the in-test `--exact` selectors keep resolving.

use super::*;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;

mod containment_publication;
mod persistence_spec;
mod scheduling_cancel_shutdown;
mod startup_cas_crash;
mod state_transitions;
mod submission_dedup;

fn request(job_id: &str, tweet_id: &str) -> ArchiveJobRequest {
    ArchiveJobRequest {
        job_id: job_id.to_owned(),
        tweet_id: tweet_id.to_owned(),
        request_id: format!("test-request-{job_id}"),
        request_json: "{}".to_owned(),
        schema_version: 1,
    }
}

fn archive_request(tweet_id: &str) -> crate::ArchiveTweetRequest {
    crate::ArchiveTweetRequest {
        tweet: xarchive_protocol::BrowserTweet {
            tweet_id: tweet_id.to_owned(),
            url: format!("https://x.com/alice/status/{tweet_id}"),
            username: Some("alice".to_owned()),
            display_name: Some("Alice".to_owned()),
            text: Some("archive me".to_owned()),
            created_at: Some("2026-09-12T00:00:00Z".to_owned()),
            tweet_type: "post".to_owned(),
            reply_to: None,
            quoted_tweet: None,
        },
        browser: None,
        profile: None,
    }
}

fn v3_spec(
    downloader_arguments: DownloaderArgumentSnapshot,
) -> (String, crate::ArchiveTweetRequest) {
    let request = archive_request("13001");
    let json = serde_json::to_string(&ExecutionSpecV3 {
        request: request.clone(),
        output_settings: OutputSettings::default(),
        downloader_arguments,
    })
    .expect("serialize v3");
    (json, request)
}

fn insert_downloaded_job<P: JobPersistence>(persistence: &mut P, job_id: &str, tweet_id: &str) {
    persistence
        .create_or_reuse(&request(job_id, tweet_id))
        .expect("create job");
    for state in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
    ] {
        persistence
            .persist_state(&JobSnapshot {
                job_id: job_id.into(),
                tweet_id: tweet_id.into(),
                state,
            })
            .expect("advance job state");
    }
}

fn complete_job_for_fixture<P: JobPersistence>(persistence: &mut P, job_id: &str, tweet_id: &str) {
    insert_downloaded_job(persistence, job_id, tweet_id);
    persistence
        .persist_state(&JobSnapshot {
            job_id: job_id.into(),
            tweet_id: tweet_id.into(),
            state: JobState::Downloaded,
        })
        .unwrap();
    persistence
        .complete_terminal_for_attempt(job_id, 0, "archives/terminal", 0)
        .unwrap();
}

fn v2_request(job_id: &str, tweet_id: &str) -> ArchiveJobRequest {
    ArchiveJobRequest {
        job_id: job_id.to_owned(),
        tweet_id: tweet_id.to_owned(),
        request_id: format!("test-request-{job_id}"),
        request_json: serde_json::to_string(&ExecutionSpecV2 {
            request: archive_request(tweet_id),
            output_settings: OutputSettings::default(),
        })
        .expect("serialize v2 execution spec"),
        schema_version: 2,
    }
}

fn insert_v2_downloaded_job(persistence: &mut StorageJobPersistence, job_id: &str, tweet_id: &str) {
    persistence
        .create_or_reuse(&v2_request(job_id, tweet_id))
        .expect("create v2 job");
    for state in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
        JobState::Downloaded,
    ] {
        persistence
            .persist_state(&JobSnapshot {
                job_id: job_id.into(),
                tweet_id: tweet_id.into(),
                state,
            })
            .expect("advance v2 job state");
    }
}

fn temporary_database_path(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!(
            "xarchive-executor-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ))
        .join("config")
        .join("archive.sqlite3")
}

fn wait_for_persisted_state(database_path: &Path, job_id: &str, expected: JobState) -> JobSnapshot {
    for _ in 0..100 {
        if let Ok(persistence) = StorageJobPersistence::open(database_path)
            && let Ok(snapshot) = persistence.snapshot(job_id)
            && snapshot.state == expected
        {
            return snapshot;
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("Job {job_id} did not reach {expected:?}");
}

#[derive(Debug)]
struct FakeJobExecution {
    /// Move-only: each `execute` consumes the stored result exactly once.
    /// A second call would panic, matching the production contract that a
    /// `JobExecutionResult`/`JobExecutionError` is consumed once and never
    /// replayed for another attempt.
    result: Option<Result<JobExecutionResult, JobExecutionError>>,
    calls: Vec<String>,
}

impl FakeJobExecution {
    fn success(files_copied: u32, archive_directory: &str) -> Self {
        Self {
            result: Some(Ok(JobExecutionResult {
                files_copied,
                archive_directory: archive_directory.to_owned(),
                download_event_recorded: false,
                state_already_updated: false,
                pending_publication: None,
            })),
            calls: Vec::new(),
        }
    }

    fn failure(error_code: &str, error_message: &str) -> Self {
        Self {
            result: Some(Err(JobExecutionError {
                error_code: error_code.to_owned(),
                error_message: error_message.to_owned(),
                persistence_already_updated: false,
            })),
            calls: Vec::new(),
        }
    }
}

impl JobExecution for FakeJobExecution {
    fn execute(
        &mut self,
        job: &JobSnapshot,
        _cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError> {
        self.calls.push(job.job_id.clone());
        self.result
            .take()
            .expect("FakeJobExecution executed more than once; results are move-only")
    }
}

struct CancellationAwareExecution;

impl JobExecution for CancellationAwareExecution {
    fn execute(
        &mut self,
        _job: &JobSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError> {
        while !cancellation.is_cancelled() {
            thread::sleep(std::time::Duration::from_millis(2));
        }
        Ok(JobExecutionResult {
            files_copied: 1,
            archive_directory: "archives/cancelled-late-result".to_owned(),
            download_event_recorded: false,
            state_already_updated: false,
            pending_publication: None,
        })
    }
}

struct DropNotifiesExecution {
    dropped: Sender<()>,
}

impl JobExecution for DropNotifiesExecution {
    fn execute(
        &mut self,
        _job: &JobSnapshot,
        _cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError> {
        Ok(JobExecutionResult {
            files_copied: 0,
            archive_directory: "archives/drop-probe".to_owned(),
            download_event_recorded: false,
            state_already_updated: false,
            pending_publication: None,
        })
    }
}

impl Drop for DropNotifiesExecution {
    fn drop(&mut self) {
        let _ = self.dropped.send(());
    }
}

/// Execution that blocks the single-active runner until `release` is set,
/// used to deterministically saturate the runner channel.
struct BlockingUntilReleased {
    release: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
}

impl JobExecution for BlockingUntilReleased {
    fn execute(
        &mut self,
        _job: &JobSnapshot,
        _cancellation: &CancellationToken,
    ) -> Result<JobExecutionResult, JobExecutionError> {
        self.started.store(true, Ordering::SeqCst);
        while !self.release.load(Ordering::SeqCst) {
            thread::sleep(std::time::Duration::from_millis(2));
        }
        Ok(JobExecutionResult {
            files_copied: 1,
            archive_directory: "archives/n1-blocker".to_owned(),
            download_event_recorded: false,
            state_already_updated: false,
            pending_publication: None,
        })
    }
}

/// Panic-safety net that releases the blocker's runner slot if a later
/// assertion panics, so the blocked thread can never hang the test binary.
struct ReleaseOnDrop(Arc<AtomicBool>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Factory that blocks the blocker Job and fails any other Job once it is
/// admitted, so the re-dispatch assertion does not depend on archive
/// finalization.
struct N1AdmissionFactory {
    release: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
}

impl JobExecutionFactory for N1AdmissionFactory {
    fn create(
        &self,
        job_id: &str,
        _snapshot: &JobSnapshot,
    ) -> Result<Box<dyn JobExecution>, ExecutorError> {
        if job_id == "job-blocker" {
            Ok(Box::new(BlockingUntilReleased {
                release: self.release.clone(),
                started: self.started.clone(),
            }))
        } else {
            Ok(Box::new(FakeJobExecution::failure(
                "TEST_EXEC_FAIL",
                "execution ran after admission",
            )))
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn startup_recovery_retries_successor_after_cas_owner_is_sigkilled() {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    const WORKER_ENV: &str = "TW2TG_V1_CAS_CRASH_WORKER";
    const ROOT_ENV: &str = "TW2TG_V1_CAS_CRASH_ROOT";

    struct KillAtSuccessorStart(PathBuf);
    impl JobExecutionFactory for KillAtSuccessorStart {
        fn create(&self, _: &str, _: &JobSnapshot) -> Result<Box<dyn JobExecution>, ExecutorError> {
            panic!("InternalV1 successor must use attempt-owned creation")
        }

        fn create_for_attempt(
            &self,
            identity: &AttemptIdentity,
            _: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            std::fs::write(&self.0, identity.attempt().to_string())
                .expect("persist post-CAS attempt marker");
            loop {
                std::thread::sleep(Duration::from_secs(30));
            }
        }

        fn create_registered_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
            _: xarchive_workflow::WriterRegistration,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            self.create_for_attempt(identity, snapshot)
        }
    }

    struct CountedSuccess(Arc<AtomicUsize>);
    impl JobExecutionFactory for CountedSuccess {
        fn create(&self, _: &str, _: &JobSnapshot) -> Result<Box<dyn JobExecution>, ExecutorError> {
            panic!("InternalV1 successor must use attempt-owned creation")
        }

        fn create_for_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            assert_eq!(
                identity.attempt(),
                3,
                "restart claims one successor after the killed owner"
            );
            assert_eq!(snapshot.state, JobState::Downloading);
            self.0.fetch_add(1, Ordering::AcqRel);
            Ok(Box::new(FakeJobExecution::success(1, "Tweets/900043")))
        }

        fn create_registered_attempt(
            &self,
            identity: &AttemptIdentity,
            snapshot: &JobSnapshot,
            _: xarchive_workflow::WriterRegistration,
        ) -> Result<Box<dyn JobExecution>, AttemptCreationError> {
            self.create_for_attempt(identity, snapshot)
        }
    }

    fn config(root: &Path) -> ExecutorConfig {
        ExecutorConfig {
            telegram_config_file: None,
            telegram: crate::config::TelegramConfig::default(),
            archive_root: root.join("archive"),
            staging_root: root.join("staging"),
            database_path: root.join("jobs.sqlite3"),
            sidecar_program: None,
            gallery_dl_program: None,
            sidecar_args: Vec::new(),
            aria2_program: None,
            use_aria2: false,
            network: ExecutorNetworkConfig::default(),
        }
    }

    if std::env::var_os(WORKER_ENV).is_some() {
        let root = PathBuf::from(std::env::var_os(ROOT_ENV).expect("fixture root"));
        let runtime = ExecutorRuntime::with_linux_containment(
            config(&root),
            LinuxExecutionContainment {
                delegated_parent: PathBuf::from("/sys/fs/cgroup"),
                python: PathBuf::from("/usr/bin/python3"),
                launcher: PathBuf::from("/missing-launcher.py"),
            },
        );
        let marker = root.join("post-cas-attempt");
        let mut runtime = runtime;
        runtime.executor = JobExecutor::with_capacity_and_factory(
            DEFAULT_QUEUE_CAPACITY,
            Arc::new(KillAtSuccessorStart(marker)),
        );
        let report = runtime
            .recover_startup()
            .expect("dispatch startup recovery")
            .join()
            .expect("startup recovery worker");
        panic!("factory must stop worker before recovery returns: {report:?}");
    }

    let temp_database = temporary_database_path("startup-v1-cas-crash");
    let root = temp_database.parent().unwrap().parent().unwrap().to_owned();
    std::fs::create_dir_all(&root).expect("fixture root");
    let database_path = root.join("jobs.sqlite3");
    let job_id = "job-startup-v1-cas-crash";
    let tweet_id = "900043";
    let mut persistence = StorageJobPersistence::open(&database_path).expect("database");
    persistence
        .create_or_reuse(&request(job_id, tweet_id))
        .expect("create job");
    persistence
        .database
        .execute_batch(&format!(
            "UPDATE jobs SET recovery_contract_version = 1 WHERE id = '{job_id}'"
        ))
        .expect("declare InternalV1");
    for state in [
        JobState::Validating,
        JobState::MetadataReady,
        JobState::Downloading,
    ] {
        persistence
            .persist_state(&JobSnapshot {
                job_id: job_id.to_owned(),
                tweet_id: tweet_id.to_owned(),
                state,
            })
            .expect("advance job state");
    }
    assert_eq!(
        persistence
            .database
            .begin_archive_attempt(job_id, "predecessor")
            .unwrap(),
        1
    );
    drop(persistence);

    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("executor::tests::startup_recovery_retries_successor_after_cas_owner_is_sigkilled")
        .arg("--nocapture")
        .env(WORKER_ENV, "1")
        .env(ROOT_ENV, &root)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn CAS owner");
    let marker = root.join("post-cas-attempt");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.exists() {
        if let Some(status) = child.try_wait().expect("poll CAS owner") {
            let output = child.wait_with_output().expect("collect worker output");
            panic!(
                "CAS owner exited before marker ({status}): {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert!(
            Instant::now() < deadline,
            "successor factory marker timeout"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "2");
    let after_cas = StorageJobPersistence::open(&database_path).expect("reopen after CAS");
    assert_eq!(after_cas.current_attempt(job_id).unwrap(), 2);
    assert_eq!(
        after_cas.snapshot(job_id).unwrap().state,
        JobState::Downloading
    );
    drop(after_cas);
    let kill = Command::new("/bin/kill")
        .args(["-KILL", &child.id().to_string()])
        .status()
        .expect("SIGKILL owner");
    assert!(kill.success());
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(child.wait().unwrap().signal(), Some(9));

    let runtime = ExecutorRuntime::with_linux_containment(
        config(&root),
        LinuxExecutionContainment {
            delegated_parent: PathBuf::from("/sys/fs/cgroup"),
            python: PathBuf::from("/usr/bin/python3"),
            launcher: PathBuf::from("/missing-launcher.py"),
        },
    );
    let executions = Arc::new(AtomicUsize::new(0));
    let mut runtime = runtime;
    runtime.executor = JobExecutor::with_capacity_and_factory(
        DEFAULT_QUEUE_CAPACITY,
        Arc::new(CountedSuccess(executions.clone())),
    );
    let report = runtime.recover_startup().unwrap().join().unwrap();
    assert!(report.fatal_error.is_none(), "startup report: {report:?}");
    assert_eq!(
        report
            .outcomes
            .iter()
            .filter(|outcome| outcome.job_id == job_id
                && outcome.status == StartupRecoveryStatus::Recovered)
            .count(),
        1
    );
    assert_eq!(executions.load(Ordering::Acquire), 1);
    let reopened = StorageJobPersistence::open(&database_path).unwrap();
    assert_eq!(reopened.current_attempt(job_id).unwrap(), 3);
    assert_eq!(reopened.snapshot(job_id).unwrap().state, JobState::Complete);
    runtime.recover_startup().unwrap().join().unwrap();
    assert_eq!(
        executions.load(Ordering::Acquire),
        1,
        "terminal successor is not executed again"
    );
    runtime.executor.shutdown().expect("shutdown runner");
    drop(reopened);
    std::fs::remove_dir_all(root).expect("remove crash fixture");
}

/// P5 integration: a killed startup-recovery owner leaves a recorded
/// writer domain and a live descendant. A fresh runtime must drain that
/// domain before making its normal recovery decision.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires systemd-run --user --property=Delegate=yes disposable cgroup"]
fn startup_recovery_drains_live_descendant_after_owner_sigkill() {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    const WORKER_ENV: &str = "TW2TG_P5_CRASH_WORKER";
    const PARENT_ENV: &str = "TW2TG_P5_DELEGATED_PARENT";
    const ROOT_ENV: &str = "TW2TG_P5_FIXTURE_ROOT";

    if std::env::var_os(WORKER_ENV).is_some() {
        let parent = PathBuf::from(std::env::var_os(PARENT_ENV).expect("delegated parent"));
        let root = PathBuf::from(std::env::var_os(ROOT_ENV).expect("fixture root"));
        let database_path = root.join("jobs.sqlite3");
        let staging_root = root.join("staging");
        let record_root = root.join("tw2tg-containment-records");
        std::fs::create_dir_all(&staging_root).expect("staging root");
        std::fs::create_dir_all(&record_root).expect("record root");
        let identity = xarchive_workflow::linux_containment::AttemptIdentity::new(
            &staging_root,
            "job-p5-crash-owner",
            1,
        )
        .unwrap()
        .with_writer("sidecar")
        .unwrap();
        let mut owner = xarchive_workflow::linux_containment::LinuxContainment::create_recorded(
            &record_root,
            &identity,
            &parent,
        )
        .expect("create recorded writer group");

        let marker = root.join("late-write-marker");
        owner.spawn_silent_with_environment(
            Path::new("/usr/bin/python3"),
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
            Path::new("/usr/bin/python3"),
            &["-c".into(), "import os,sys,time; from pathlib import Path; Path(sys.argv[1]+'.ready').write_text(str(os.getpid())); time.sleep(8); Path(sys.argv[1]).write_text('late')".into(), marker.to_string_lossy().into_owned()],
            &[], &[],
        ).expect("launch contained late writer");
        let child_ready = marker.with_extension("ready");
        let ready_deadline = Instant::now() + Duration::from_secs(3);
        while !child_ready.exists() {
            assert!(Instant::now() < ready_deadline, "descendant must start");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(owner.poll_exit().expect("poll descendant").is_none());

        let mut database = StorageJobPersistence::open(&database_path).expect("database");
        database
            .create_or_reuse(&request("job-p5-crash-owner", "900051"))
            .expect("create InternalV1 job");
        for state in [
            JobState::Validating,
            JobState::MetadataReady,
            JobState::Downloading,
        ] {
            database
                .persist_state(&JobSnapshot {
                    job_id: "job-p5-crash-owner".into(),
                    tweet_id: "900051".into(),
                    state,
                })
                .expect("advance job to active writer state");
        }

        database
            .database
            .execute_batch(
                "UPDATE jobs SET recovery_contract_version = 1 WHERE id = 'job-p5-crash-owner'",
            )
            .expect("declare the durable InternalV1 recovery contract");
        drop(database);
        Database::open(&database_path)
            .expect("database")
            .begin_archive_attempt("job-p5-crash-owner", "p5-crash-test")
            .expect("begin archive attempt");

        std::fs::write(
            root.join("owner.ready"),
            owner.path().as_os_str().as_encoded_bytes(),
        )
        .expect("persist readiness");
        use std::io::Write as _;
        std::io::stdout().flush().expect("flush readiness");
        loop {
            std::thread::sleep(Duration::from_secs(60));
        }
    }

    let membership = std::fs::read_to_string("/proc/self/cgroup").expect("cgroup membership");
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .expect("unified cgroup membership");
    let delegated_parent = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    assert!(
        delegated_parent.join("cgroup.procs").metadata().is_ok(),
        "test must run inside an explicitly delegated cgroup"
    );
    let fixture_database = temporary_database_path("p5-crash-recovery");
    let root = fixture_database.parent().unwrap().to_owned();
    std::fs::create_dir_all(&root).expect("fixture root");

    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("executor::tests::startup_recovery_drains_live_descendant_after_owner_sigkill")
        .arg("--nocapture")
        .arg("--ignored")
        .env(WORKER_ENV, "1")
        .env(PARENT_ENV, &delegated_parent)
        .env(ROOT_ENV, &root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn crash owner worker");
    let ready_deadline = Instant::now() + Duration::from_secs(6);
    while !root.join("owner.ready").exists() {
        if let Some(status) = child.try_wait().expect("poll owner") {
            let output = child.wait_with_output().expect("owner output");
            panic!(
                "owner exited {status}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        if Instant::now() >= ready_deadline {
            child.kill().expect("kill unready owner");
            let output = child.wait_with_output().expect("owner output");
            panic!(
                "owner readiness timed out: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let group_path = PathBuf::from(std::fs::read_to_string(root.join("owner.ready")).unwrap());
    assert!(
        std::fs::read_to_string(group_path.join("cgroup.events"))
            .expect("live writer domain")
            .lines()
            .any(|line| line == "populated 1"),
        "crash fixture must have a live contained writer"
    );
    let status = Command::new("/bin/kill")
        .args(["-KILL", &child.id().to_string()])
        .status()
        .expect("SIGKILL owner");
    assert!(status.success(), "SIGKILL command succeeded");
    let killed = child.wait().expect("reap killed owner");
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(killed.signal(), Some(9), "owner must die from SIGKILL");
    assert!(
        std::fs::read_to_string(group_path.join("cgroup.events"))
            .expect("orphan writer domain")
            .lines()
            .any(|line| line == "populated 1"),
        "writer must survive owner death until successor drain"
    );

    let database_path = root.join("jobs.sqlite3");
    let staging_root = root.join("staging");
    let config = ExecutorConfig {
        telegram_config_file: None,
        telegram: crate::config::TelegramConfig::default(),
        archive_root: root.join("archive"),
        staging_root: staging_root.clone(),
        database_path: database_path.clone(),
        sidecar_program: None,
        gallery_dl_program: None,
        sidecar_args: Vec::new(),
        aria2_program: None,
        use_aria2: false,
        network: ExecutorNetworkConfig::default(),
    };
    let runtime = ExecutorRuntime::with_linux_containment(
        config,
        LinuxExecutionContainment {
            delegated_parent,
            python: PathBuf::from("/usr/bin/python3"),
            launcher: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../sidecar/src/xarchive_downloader/cgroup_launcher.py"),
        },
    );
    let recovery = runtime
        .recover_startup()
        .expect("startup recovery dispatch");
    let report = recovery.join().expect("startup recovery report");
    assert!(report.fatal_error.is_none());
}

/// Test persistence that reports a durable archive commit boundary, either
/// at the pre-check (like a already-PREPARED journal observed before the
/// writer stops) or only at decision time (like a prepare landing between
/// the check and the decision).
#[derive(Default)]
struct CommitBoundaryPersistence {
    inner: InMemoryJobPersistence,
    report_at_precheck: bool,
    report_at_decision: bool,
}

impl JobPersistence for CommitBoundaryPersistence {
    fn create_or_reuse(
        &mut self,
        request: &ArchiveJobRequest,
    ) -> Result<SubmitResult, ExecutorError> {
        self.inner.create_or_reuse(request)
    }
    fn snapshot(&self, job_id: &str) -> Result<JobSnapshot, ExecutorError> {
        self.inner.snapshot(job_id)
    }
    fn list_recovery_candidates(&self) -> Result<Vec<JobSnapshot>, ExecutorError> {
        self.inner.list_recovery_candidates()
    }
    fn persist_state(&mut self, snapshot: &JobSnapshot) -> Result<(), ExecutorError> {
        self.inner.persist_state(snapshot)
    }
    fn fail(
        &mut self,
        job_id: &str,
        error_code: &str,
        error_message: &str,
    ) -> Result<JobSnapshot, ExecutorError> {
        self.inner.fail(job_id, error_code, error_message)
    }
    fn record_event(&mut self, job_id: &str, event: ExecutorEvent) -> Result<(), ExecutorError> {
        self.inner.record_event(job_id, event)
    }
    fn events(&self, job_id: &str) -> Result<Vec<ExecutorEvent>, ExecutorError> {
        self.inner.events(job_id)
    }
    fn begin_attempt(&mut self, job_id: &str) -> Result<u32, ExecutorError> {
        self.inner.begin_attempt(job_id)
    }
    fn current_attempt(&self, job_id: &str) -> Result<u32, ExecutorError> {
        self.inner.current_attempt(job_id)
    }
    fn claim_attempt(
        &mut self,
        job_id: &str,
        expected_attempt: u32,
        from: JobState,
        to: JobState,
    ) -> Result<Option<u32>, ExecutorError> {
        self.inner.claim_attempt(job_id, expected_attempt, from, to)
    }
    fn record_claimed_attempt_state(
        &mut self,
        job_id: &str,
        from: JobState,
        to: JobState,
    ) -> Result<(), ExecutorError> {
        self.inner.record_claimed_attempt_state(job_id, from, to)
    }
    fn attempt_is_current(&self, job_id: &str, attempt: u32) -> Result<bool, ExecutorError> {
        self.inner.attempt_is_current(job_id, attempt)
    }
    fn complete_terminal_for_attempt(
        &mut self,
        job_id: &str,
        attempt: u32,
        archive_directory: &str,
        files_copied: u32,
    ) -> Result<JobSnapshot, ExecutorError> {
        self.inner
            .complete_terminal_for_attempt(job_id, attempt, archive_directory, files_copied)
    }
    fn archive_commit_started(&self, _job_id: &str) -> Result<bool, ExecutorError> {
        Ok(self.report_at_precheck)
    }
    fn persist_cancellation(&mut self, job_id: &str) -> Result<CancelArbitration, ExecutorError> {
        if self.report_at_decision {
            return Ok(CancelArbitration::CommitWins);
        }
        self.inner.persist_cancellation(job_id)
    }
}
