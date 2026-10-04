//! Explicit control requests never retry: a lost response may mean success.
use xarchive_telegram::{BotToken, EndpointMode, TelegramRequest, TelegramTransport};

pub(crate) fn diagnostic_request(
    operation: &str,
    confirmed: bool,
    config: &crate::config::TelegramConfig,
) -> Result<TelegramRequest, String> {
    match operation {
        "auth" => Ok(TelegramRequest::GetMe),
        "target" => Ok(TelegramRequest::GetChat {
            chat_id: config.chat_id.clone(),
        }),
        "message" if confirmed && !config.migration_pending => Ok(TelegramRequest::Message(
            xarchive_telegram::SendMessageRequest {
                chat_id: config.chat_id.clone(),
                message_thread_id: config.message_thread_id,
                text: "XArchive explicit connection test".into(),
                disable_web_page_preview: true,
            },
        )),
        _ => {
            Err("unsupported operation, migration pending or explicit confirmation missing".into())
        }
    }
}

pub(crate) fn migrate(
    source: &dyn TelegramTransport,
    target: &dyn TelegramTransport,
    token: &BotToken,
    mode: EndpointMode,
    expected_identity: &str,
    confirmed: bool,
) -> Result<(), String> {
    if !confirmed {
        return Err("explicit migration confirmation required".into());
    }
    let control = |request| -> Result<(), String> {
        let response = source.send(token, request).map_err(|_| {
            "migration control failed or outcome unknown; sender stays paused; review before retry"
                .to_owned()
        })?;
        if !response.ok || response.result != Some(serde_json::Value::Bool(true)) {
            return Err("migration control was not confirmed; sender stays paused".into());
        }
        Ok(())
    };
    match mode {
        EndpointMode::Cloud => control(TelegramRequest::LogOut)?,
        EndpointMode::Local => {
            control(TelegramRequest::DeleteWebhook)?;
            control(TelegramRequest::Close)?;
        }
    }
    let identity = xarchive_telegram::verify_bot_identity(target, token)
        .map_err(|_| "target authentication failed; sender stays paused".to_owned())?;
    if identity != expected_identity {
        return Err("target bot identity mismatch; sender stays paused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use xarchive_telegram::{TelegramError, TelegramResponse};
    struct Fake {
        requests: RefCell<Vec<TelegramRequest>>,
        fail: bool,
        identity: i64,
    }
    impl TelegramTransport for Fake {
        fn send(
            &self,
            _: &BotToken,
            request: TelegramRequest,
        ) -> Result<TelegramResponse, TelegramError> {
            let auth = matches!(request, TelegramRequest::GetMe);
            self.requests.borrow_mut().push(request);
            if self.fail {
                return Err(TelegramError::ResponseLost);
            }
            Ok(TelegramResponse {
                ok: true,
                error_code: None,
                description: None,
                parameters: None,
                result: Some(if auth {
                    serde_json::json!({"id": self.identity, "is_bot": true})
                } else {
                    serde_json::json!(true)
                }),
            })
        }
    }
    fn fake(fail: bool, identity: i64) -> Fake {
        Fake {
            requests: RefCell::new(vec![]),
            fail,
            identity,
        }
    }

    #[test]
    fn diagnostics_cannot_logout_close_or_send_without_confirmation() {
        let mut config = crate::config::TelegramConfig::default();
        for operation in ["logOut", "close", "deleteWebhook", "message", "invalid"] {
            assert!(diagnostic_request(operation, false, &config).is_err());
        }
        assert_eq!(
            diagnostic_request("auth", false, &config).unwrap(),
            TelegramRequest::GetMe
        );
        assert!(diagnostic_request("message", true, &config).is_ok());
        config.migration_pending = true;
        assert!(diagnostic_request("message", true, &config).is_err());
    }
    #[test]
    fn confirmation_and_unknown_outcome_never_retry() {
        let source = fake(true, 1);
        let target = fake(false, 1);
        let token = BotToken::new("123:secret").unwrap();
        assert!(
            migrate(
                &source,
                &target,
                &token,
                EndpointMode::Cloud,
                "telegram-bot:1",
                false
            )
            .is_err()
        );
        assert!(source.requests.borrow().is_empty());
        assert!(
            migrate(
                &source,
                &target,
                &token,
                EndpointMode::Cloud,
                "telegram-bot:1",
                true
            )
            .is_err()
        );
        assert_eq!(source.requests.borrow().len(), 1);
        assert!(target.requests.borrow().is_empty());
    }
    #[test]
    fn local_removes_webhook_before_close_and_checks_identity() {
        let source = fake(false, 1);
        let target = fake(false, 2);
        assert!(
            migrate(
                &source,
                &target,
                &BotToken::new("123:secret").unwrap(),
                EndpointMode::Local,
                "telegram-bot:1",
                true
            )
            .is_err()
        );
        assert_eq!(
            *source.requests.borrow(),
            vec![TelegramRequest::DeleteWebhook, TelegramRequest::Close]
        );
    }

    #[test]
    fn cloud_success_checks_target_once_without_sending_content() {
        let source = fake(false, 1);
        let target = fake(false, 1);
        migrate(
            &source,
            &target,
            &BotToken::new("123:secret").unwrap(),
            EndpointMode::Cloud,
            "telegram-bot:1",
            true,
        )
        .unwrap();
        assert_eq!(*source.requests.borrow(), vec![TelegramRequest::LogOut]);
        assert_eq!(*target.requests.borrow(), vec![TelegramRequest::GetMe]);
    }

    #[test]
    fn local_first_control_failure_does_not_close_or_authenticate_target() {
        let source = fake(true, 1);
        let target = fake(false, 1);
        assert!(
            migrate(
                &source,
                &target,
                &BotToken::new("123:secret").unwrap(),
                EndpointMode::Local,
                "telegram-bot:1",
                true
            )
            .is_err()
        );
        assert_eq!(
            *source.requests.borrow(),
            vec![TelegramRequest::DeleteWebhook]
        );
        assert!(target.requests.borrow().is_empty());
    }
}
