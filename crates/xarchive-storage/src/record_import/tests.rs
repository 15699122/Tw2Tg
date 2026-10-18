#[cfg(test)]
mod tests {

    use xarchive_core::{ArchiveAuthor, ArchiveMedia, ArchiveMetadata, DownloadRecord};

    use crate::{
        Database, StorageError,
        record_import::{AcceptedRecord, RecordImportOutcome},
    };

    fn memory_database() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn sample_record(handle: &str, id: &str, text: &str) -> DownloadRecord {
        DownloadRecord {
            account_handle: handle.into(),
            metadata: ArchiveMetadata {
                schema_version: 1,
                tweet_id: id.into(),
                url: format!("https://x.test/i/status/{id}"),
                tweet_type: "original".into(),
                author: ArchiveAuthor {
                    user_id: Some(format!("u-{handle}")),
                    username: Some(handle.into()),
                    display_name: None,
                },
                created_at: None,
                text: text.into(),
                media: vec![ArchiveMedia {
                    index: 1,
                    media_id: None,
                    media_type: "photo".into(),
                    file: "media/a.jpg".into(),
                    mime_type: None,
                    size_bytes: 7,
                    sha256: "d".into(),
                }],
                archived_at: "2026-10-17T00:00:00Z".into(),
                reply_to: None,
                quoted_tweet: None,
            },
        }
    }

    fn accepted_all(count: usize) -> Vec<AcceptedRecord> {
        (0..count).map(|index| AcceptedRecord { index }).collect()
    }

    fn stored_json(database: &Database, id: &str) -> Option<String> {
        database.merged_metadata_json(id).unwrap()
    }

    #[test]
    fn ledger_migration_upgrades_existing_database_without_rewriting_records() {
        let path = std::env::temp_dir().join(format!(
            "tw2tg-ledger-migration-{}.sqlite",
            std::process::id()
        ));
        let record = sample_record("alice", "820", "legacy");
        {
            let database = Database::open(&path).unwrap();
            database
                .import_accepted_records(
                    std::slice::from_ref(&record),
                    &accepted_all(1),
                    "now",
                    "/archive",
                )
                .unwrap();
            database.connection_for_test().execute_batch("DROP TABLE record_exchange_ledger; DELETE FROM schema_migrations WHERE version = 20;").unwrap();
        }
        {
            let database = Database::open(&path).unwrap();
            assert_eq!(
                stored_json(&database, "820").unwrap(),
                serde_json::to_string(&record.metadata).unwrap()
            );
            assert!(
                database
                    .exchanged_record_json(xarchive_core::DOWNLOAD_RECORDS_KIND, "alice", "820")
                    .unwrap()
                    .is_none()
            );
            assert!(matches!(
                database
                    .import_accepted_records(&[record], &accepted_all(1), "later", "/archive")
                    .unwrap(),
                RecordImportOutcome::Committed {
                    accepted: 0,
                    identical: 1
                }
            ));
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn download_preview_matches_ledger_payload_and_root_conflicts() {
        let database = memory_database();
        let record = sample_record("collector", "819", "original");
        database
            .import_accepted_records(
                std::slice::from_ref(&record),
                &accepted_all(1),
                "now",
                "/archive",
            )
            .unwrap();
        assert!(
            database
                .download_record_conflict(&record, "/archive")
                .unwrap()
                .is_none()
        );
        assert!(
            database
                .download_record_conflict(&record, "/other")
                .unwrap()
                .is_some()
        );
        assert!(matches!(
            database
                .import_accepted_records(
                    std::slice::from_ref(&record),
                    &accepted_all(1),
                    "now",
                    "/other"
                )
                .unwrap(),
            RecordImportOutcome::Conflicts { .. }
        ));
        let mut changed = record.clone();
        changed.metadata.text = "changed".into();
        assert!(
            database
                .download_record_conflict(&changed, "/archive")
                .unwrap()
                .is_some()
        );
        assert!(matches!(
            database
                .import_accepted_records(&[changed], &accepted_all(1), "now", "/archive")
                .unwrap(),
            RecordImportOutcome::Conflicts { .. }
        ));
        assert_eq!(
            stored_json(&database, "819").unwrap(),
            serde_json::to_string(&record.metadata).unwrap()
        );
    }

    #[test]
    fn absent_records_commit_users_tweets_and_metadata_in_one_transaction() {
        let database = memory_database();
        let records = vec![
            sample_record("alice", "101", "one"),
            sample_record("bob", "102", "two"),
        ];
        let accepted = accepted_all(records.len());
        let outcome = database
            .import_accepted_records(&records, &accepted, "t0", "/archive")
            .unwrap();
        assert_eq!(
            outcome,
            RecordImportOutcome::Committed {
                accepted: 2,
                identical: 0
            }
        );
        for (id, handle) in [("101", "alice"), ("102", "bob")] {
            let stored = stored_json(&database, id).expect("tweet must exist");
            assert!(
                stored.contains(id),
                "merged payload must carry the tweet id"
            );
            let user_id: String = database.connection_for_test().query_row(
                "SELECT u.x_user_id FROM users u JOIN tweets t ON t.user_id = u.id WHERE t.tweet_id = ?1",
                [id], |row| row.get(0),
            ).unwrap();
            assert_eq!(user_id, format!("u-{handle}"));
        }
    }

    #[test]
    fn download_then_archive_completes_facts_and_sql_failure_is_atomic() {
        let database = memory_database();
        let root =
            std::env::temp_dir().join(format!("tw2tg-ledger-upgrade-{}", std::process::id()));
        std::fs::create_dir_all(root.join("Tweets/811")).unwrap();
        let download = sample_record("alice", "811", "one");
        database
            .import_accepted_records(
                std::slice::from_ref(&download),
                &accepted_all(1),
                "t0",
                root.to_str().unwrap(),
            )
            .unwrap();
        let record = xarchive_core::ArchiveRecord {
            account_handle: download.account_handle,
            metadata: download.metadata,
            final_path: "Tweets/811".into(),
        };
        database.connection_for_test().execute_batch("CREATE TRIGGER fail_ledger BEFORE INSERT ON record_exchange_ledger WHEN NEW.kind = 'tw2tg.archive-records' BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
        assert!(
            database
                .import_accepted_archive_records(
                    std::slice::from_ref(&record),
                    &accepted_all(1),
                    "t1",
                    &root
                )
                .is_err()
        );
        assert_eq!(
            database
                .tweet_archive_facts("811")
                .unwrap()
                .unwrap()
                .archive_directory,
            root.to_str().unwrap()
        );
        assert!(
            database
                .tweet_archive_facts("811")
                .unwrap()
                .unwrap()
                .media
                .is_empty()
        );
        database
            .connection_for_test()
            .execute_batch("DROP TRIGGER fail_ledger")
            .unwrap();
        let document = xarchive_core::ArchiveRecordsDocument::new(vec![record.clone()]);
        let yaml =
            xarchive_core::ArchiveRecordsDocument::from_yaml(&document.to_yaml().unwrap()).unwrap();
        assert_eq!(
            database
                .import_accepted_archive_records(&yaml.records, &accepted_all(1), "t2", &root)
                .unwrap(),
            RecordImportOutcome::Committed {
                accepted: 1,
                identical: 0
            }
        );
        assert_eq!(
            database
                .import_accepted_archive_records(&document.records, &accepted_all(1), "t3", &root)
                .unwrap(),
            RecordImportOutcome::Committed {
                accepted: 0,
                identical: 1
            }
        );
        assert_eq!(
            database
                .tweet_archive_facts("811")
                .unwrap()
                .unwrap()
                .media
                .len(),
            1
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archive_record_ledger_preserves_final_path_and_conflicts_roll_back() {
        let database = memory_database();
        let root = std::env::temp_dir().join(format!("tw2tg-ledger-{}", std::process::id()));
        std::fs::create_dir_all(root.join("Tweets/801")).unwrap();
        std::fs::create_dir_all(root.join("Tweets/802")).unwrap();
        let download = sample_record("alice", "801", "one");
        let record = xarchive_core::ArchiveRecord {
            account_handle: download.account_handle,
            metadata: download.metadata,
            final_path: "Tweets/801".into(),
        };
        assert_eq!(
            database
                .import_accepted_archive_records(
                    std::slice::from_ref(&record),
                    &accepted_all(1),
                    "t0",
                    &root
                )
                .unwrap(),
            RecordImportOutcome::Committed {
                accepted: 1,
                identical: 0
            }
        );
        assert_eq!(
            database
                .import_accepted_archive_records(
                    std::slice::from_ref(&record),
                    &accepted_all(1),
                    "t1",
                    &root
                )
                .unwrap(),
            RecordImportOutcome::Committed {
                accepted: 0,
                identical: 1
            }
        );
        let raw: String = database
            .connection_for_test()
            .query_row(
                "SELECT canonical_record_json FROM record_exchange_ledger",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<xarchive_core::ArchiveRecord>(&raw).unwrap(),
            record
        );
        let facts = database.tweet_archive_facts("801").unwrap().unwrap();
        assert_eq!(
            facts.archive_directory,
            root.join("Tweets/801").to_str().unwrap()
        );
        assert_eq!(facts.media.len(), 1);
        assert_eq!(facts.media[0].relative_path, "media/a.jpg");
        let mut fresh = record.clone();
        fresh.metadata.tweet_id = "802".into();
        fresh.final_path = "Tweets/802".into();
        let mut conflicting = record.clone();
        conflicting.final_path = "Tweets/802".into();
        assert!(
            database
                .archive_record_conflict(&conflicting, &root)
                .unwrap()
                .is_some()
        );
        assert!(
            database
                .archive_record_conflict(&record, &root)
                .unwrap()
                .is_none()
        );
        assert!(matches!(
            database
                .import_accepted_archive_records(
                    &[fresh, conflicting],
                    &accepted_all(2),
                    "t2",
                    &root
                )
                .unwrap(),
            RecordImportOutcome::Conflicts { .. }
        ));
        assert!(database.merged_metadata_json("802").unwrap().is_none());
        let count: i64 = database
            .connection_for_test()
            .query_row("SELECT count(*) FROM record_exchange_ledger", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
        database
            .connection_for_test()
            .execute("UPDATE media SET sha256 = 'conflict'", [])
            .unwrap();
        assert!(
            database
                .archive_record_conflict(&record, &root)
                .unwrap()
                .is_some()
        );
        assert!(matches!(
            database
                .import_accepted_archive_records(
                    std::slice::from_ref(&record),
                    &accepted_all(1),
                    "t3",
                    &root
                )
                .unwrap(),
            RecordImportOutcome::Conflicts { .. }
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn identical_reimport_is_a_noop_and_conflict_rejects_without_committing() {
        let database = memory_database();
        let records = vec![sample_record("alice", "201", "one")];
        let accepted = accepted_all(records.len());
        let first = database
            .import_accepted_records(&records, &accepted, "t0", "/archive")
            .unwrap();
        assert_eq!(
            first,
            RecordImportOutcome::Committed {
                accepted: 1,
                identical: 0
            }
        );

        let second = database
            .import_accepted_records(&records, &accepted, "t1", "/archive")
            .unwrap();
        assert_eq!(
            second,
            RecordImportOutcome::Committed {
                accepted: 0,
                identical: 1
            }
        );

        let mut changed = records.clone();
        changed[0].metadata.text = "changed".into();
        let outcome = database
            .import_accepted_records(&changed, &accepted, "t2", "/archive")
            .unwrap();
        assert!(matches!(outcome, RecordImportOutcome::Conflicts { .. }));
        let stored = stored_json(&database, "201").expect("tweet must still exist");
        assert!(
            stored.contains("\"text\":\"one\""),
            "conflict must not overwrite"
        );
    }

    #[test]
    fn mixed_accept_and_conflict_aborts_the_whole_transaction() {
        let database = memory_database();
        let first = sample_record("alice", "301", "one");
        assert_eq!(
            database
                .import_accepted_records(
                    std::slice::from_ref(&first),
                    &accepted_all(1),
                    "t0",
                    "/archive"
                )
                .unwrap(),
            RecordImportOutcome::Committed {
                accepted: 1,
                identical: 0
            }
        );

        let new_record = sample_record("bob", "302", "two");
        let mut conflicting = first.clone();
        conflicting.metadata.text = "changed".into();
        let records = vec![new_record, conflicting];
        let accepted = accepted_all(2);
        let outcome = database
            .import_accepted_records(&records, &accepted, "t1", "/archive")
            .unwrap();
        assert!(matches!(outcome, RecordImportOutcome::Conflicts { .. }));
        assert!(
            database.tweet_row_id("302").is_err(),
            "the accepted prefix must not persist when a later record conflicts"
        );
        let stored = stored_json(&database, "301").expect("original must survive");
        assert!(stored.contains("\"text\":\"one\""));
    }

    #[test]
    fn out_of_range_index_returns_invalid_metadata_error() {
        let database = memory_database();
        let records = vec![sample_record("alice", "601", "one")];
        let accepted = vec![AcceptedRecord { index: 5 }];
        let error = database
            .import_accepted_records(&records, &accepted, "t0", "/archive")
            .unwrap_err();
        assert!(matches!(error, StorageError::InvalidMetadata(_)));
    }

    #[test]
    fn commit_error_rolls_back_whole_transaction() {
        let database = memory_database();
        let record1 = sample_record("alice", "701", "one");
        let record2 = sample_record("bob", "702", "two");

        let transaction = database
            .connection_for_test()
            .unchecked_transaction()
            .unwrap();

        let incoming1 = serde_json::to_string(&record1.metadata).unwrap();
        Database::commit_imported_record(&transaction, &record1, &incoming1, "t0", "/archive")
            .unwrap();

        let result = Database::commit_imported_record(
            &transaction,
            &record2,
            "{invalid json",
            "t1",
            "/archive",
        );
        assert!(result.is_err());

        let _ = transaction.rollback();

        assert!(
            database.merged_metadata_json("701").unwrap().is_none(),
            "rolled-back record must not persist"
        );
        assert!(database.tweet_row_id("701").is_err());
        assert!(database.tweet_row_id("702").is_err());
    }
}
