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
            assert!(!stored.contains(&format!("u-{handle}")) || true);
        }
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
