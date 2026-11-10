//! Formatting and send-planning suites: media grouping, stable keys and the
//! outbox fingerprint projection.

use super::*;

#[test]
fn media_planner_splits_document_runs_without_reordering() {
    let mut items: Vec<_> = (0..5).map(planned_item).collect();
    items[1].media_kind = MediaKind::Video;
    items[2].media_kind = MediaKind::Document;
    items[3].media_kind = MediaKind::Document;
    let plans = plan_media_sends("-100", Some(42), items.clone(), "mixed");
    assert_eq!(plans.len(), 3);
    let groups: Vec<_> = plans
        .iter()
        .map(|plan| match &plan.payload {
            SendPayload::Media { items, .. } => items.clone(),
            _ => panic!("media plan expected"),
        })
        .collect();
    assert_eq!(
        groups.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![2, 2, 1]
    );
    assert_eq!(groups.into_iter().flatten().collect::<Vec<_>>(), items);
    assert_eq!(plans[2].message_kind, "media_single");
}

fn planned_item(index: usize) -> PlannedMediaItem {
    PlannedMediaItem {
        media_kind: MediaKind::Photo,
        file_path: PathBuf::from(format!("archive/media/{index}.jpg")),
        file_name: format!("x_00_{index}.jpg"),
        mime_type: Some("image/jpeg".to_owned()),
        caption: None,
        content_sha256: Some(format!("sha-{index}")),
        size_bytes: Some(1024),
    }
}

#[test]
fn media_plans_promote_a_trailing_single_and_keep_stable_keys() {
    // One item: a single send, never a one-item album.
    let single = plan_media_sends("-1001", None, vec![planned_item(0)], "tweet-1:media");
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].message_kind, "media_single");
    assert_eq!(single[0].idempotency_key, "tweet-1:media:00");

    // Two items: one album with both, in order.
    let pair = plan_media_sends(
        "-1001",
        None,
        (0..2).map(planned_item).collect(),
        "tweet-1:media",
    );
    assert_eq!(pair.len(), 1);
    assert_eq!(pair[0].message_kind, "media_album");
    match &pair[0].payload {
        SendPayload::Media { items, .. } => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].file_name, "x_00_0.jpg");
            assert_eq!(items[1].file_name, "x_00_1.jpg");
        }
        other => panic!("expected a media payload, got {other:?}"),
    }

    // Eleven items: an album of ten plus a promoted single.
    let eleven = plan_media_sends(
        "-1001",
        Some(7),
        (0..11).map(planned_item).collect(),
        "tweet-1:media",
    );
    assert_eq!(eleven.len(), 2);
    assert_eq!(eleven[0].message_kind, "media_album");
    assert_eq!(eleven[1].message_kind, "media_single");
    assert_eq!(eleven[0].idempotency_key, "tweet-1:media:00");
    assert_eq!(eleven[1].idempotency_key, "tweet-1:media:10");
    match &eleven[1].payload {
        SendPayload::Media {
            message_thread_id,
            items,
            ..
        } => {
            assert_eq!(*message_thread_id, Some(7));
            assert_eq!(items.len(), 1);
        }
        other => panic!("expected a media payload, got {other:?}"),
    }

    // Twenty-one items: two albums plus a trailing single.
    assert_eq!(
        plan_media_sends(
            "-1001",
            None,
            (0..21).map(planned_item).collect(),
            "t:media"
        )
        .len(),
        3
    );
    // The same input always produces the same plan, so a restart never
    // queues the same content twice.
    assert_eq!(
        plan_media_sends(
            "-1001",
            Some(7),
            (0..11).map(planned_item).collect(),
            "tweet-1:media"
        ),
        eleven
    );
}

#[test]
fn a_planned_send_fingerprints_its_content_and_becomes_an_outbox_entry() {
    let plan = plan_media_sends(
        "-1001",
        None,
        (0..2).map(planned_item).collect(),
        "tweet-1:media",
    )
    .remove(0);
    let fingerprint = plan.fingerprint();
    assert_eq!(fingerprint.len(), 64, "SHA-256 hex");
    assert_eq!(fingerprint, plan.fingerprint(), "fingerprints are stable");

    // Changing the content changes the fingerprint, so a recorded result
    // can never be reused for different media.
    let mut changed = plan.clone();
    if let SendPayload::Media { items, .. } = &mut changed.payload {
        items[1].content_sha256 = Some("sha-other".to_owned());
    }
    assert_ne!(changed.fingerprint(), fingerprint);

    let mut moved = plan.clone();
    if let SendPayload::Media { items, .. } = &mut moved.payload {
        items[0].file_path = PathBuf::from("relocated/0.jpg");
    }
    assert_eq!(moved.fingerprint(), fingerprint);

    let entry = plan
        .to_outbox_entry(
            "bot-identity",
            "archive",
            Some(42),
            7,
            1,
            0,
            "2026-10-01T00:00:00Z",
        )
        .expect("safe relative media reference");
    assert_eq!(entry.target_scope, "-1001:none");
    assert_eq!(entry.payload_schema_version, 1);
    assert!(!entry.payload_json.contains("/tmp/"));
    assert!(entry.payload_json.contains("media/0.jpg"));
    assert_eq!(entry.bot_identity, "bot-identity");
    assert_eq!(entry.chat_id, "-1001");
    assert_eq!(entry.idempotency_key, "tweet-1:media:00");
    assert_eq!(entry.request_fingerprint, fingerprint);
    assert_eq!(entry.message_kind, "media_album");
    assert_eq!(
        entry.config_version, 7,
        "queued items bind to a settings version"
    );
    assert_eq!(entry.plan_order, 0);
    assert_eq!(entry.tweet_id, Some(42));
    assert_eq!(
        entry.media_reference.as_deref(),
        Some("archive/media/0.jpg;archive/media/1.jpg")
    );
    assert_eq!(entry.content_sha256.as_deref(), Some("sha-0"));

    // A text plan carries no media reference and keeps its own kind.
    let text = plan_text_send("-1001", Some(9), "hello", "tweet-1:metadata");
    assert_eq!(text.message_kind, "message");
    let text_entry = text
        .to_outbox_entry(
            "bot-identity",
            "archive",
            None,
            7,
            1,
            1,
            "2026-10-01T00:00:00Z",
        )
        .expect("text plan");
    assert_eq!(text_entry.media_reference, None);
    assert_eq!(text_entry.content_sha256, None);
    assert_eq!(text_entry.message_thread_id, Some(9));
    assert_eq!(text_entry.idempotency_key, "tweet-1:metadata");
}
