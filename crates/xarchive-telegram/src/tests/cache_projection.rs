//! File-cache suites: conservative invalid-file-id detection.

use super::*;

#[test]
fn invalid_file_id_detection_is_conservative() {
    for description in [
        "Bad Request: invalid file id",
        "Bad Request: wrong file identifier specified",
        "Bad Request: file reference expired",
    ] {
        let error = TelegramError::Api {
            code: 400,
            description: description.into(),
            parameters: None,
        };
        assert!(
            is_invalid_file_id_error(&error),
            "{description} must trigger the upload fallback"
        );
    }
    // Everything else keeps the cache: permission, network, target and
    // generic server failures are never cache invalidation.
    for error in [
        TelegramError::Api {
            code: 400,
            description: "Bad Request: chat not found".into(),
            parameters: None,
        },
        TelegramError::Api {
            code: 403,
            description: "Forbidden: bot was blocked".into(),
            parameters: None,
        },
        TelegramError::Api {
            code: 429,
            description: "Too Many Requests".into(),
            parameters: None,
        },
        TelegramError::Transport("connection failed".into()),
        TelegramError::ResponseLost,
    ] {
        assert!(
            !is_invalid_file_id_error(&error),
            "{error:?} must keep the cache"
        );
    }
}
