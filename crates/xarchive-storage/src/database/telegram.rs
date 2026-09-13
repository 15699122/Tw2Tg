use rusqlite::{OptionalExtension, params};

use crate::*;

use xarchive_telegram::{
    PendingSendRecord, SendState, SendStateError, SendStateStore, SentSendRecord,
};

fn send_state_error(error: rusqlite::Error) -> SendStateError {
    SendStateError::Store(error.to_string())
}

impl SendStateStore for Database {
    fn find_sent(
        &self,
        chat_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<SentSendRecord>, SendStateError> {
        self.connection
            .query_row(
                "SELECT chat_id, idempotency_key, message_kind, telegram_message_id, telegram_file_id \
                 FROM telegram_send_attempts \
                 WHERE chat_id = ?1 AND idempotency_key = ?2 AND state = 'SENT'",
                params![chat_id, idempotency_key],
                |row| {
                    Ok(SentSendRecord {
                        chat_id: row.get(0)?,
                        idempotency_key: row.get(1)?,
                        message_kind: row.get(2)?,
                        telegram_message_id: row.get(3)?,
                        telegram_file_id: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(send_state_error)
    }

    fn record_pending(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        message_kind: &str,
        updated_at: &str,
    ) -> Result<(), SendStateError> {
        let changed = self
            .connection
            .execute(
                "INSERT INTO telegram_send_attempts \
                     (chat_id, idempotency_key, message_kind, state, attempt_count, created_at, updated_at) \
                     VALUES (?1, ?2, ?3, 'PENDING', 1, ?4, ?4) \
                 ON CONFLICT(chat_id, idempotency_key) DO UPDATE SET \
                     state = 'PENDING', \
                     attempt_count = attempt_count + 1, \
                     telegram_message_id = NULL, \
                     telegram_file_id = NULL, \
                     last_error_code = NULL, \
                     last_error_message = NULL, \
                     updated_at = excluded.updated_at",
                params![chat_id, idempotency_key, message_kind, updated_at],
            )
            .map_err(send_state_error)?;
        if changed == 0 {
            return Err(SendStateError::Store(
                "telegram_send_attempts insert did not apply".to_owned(),
            ));
        }
        Ok(())
    }

    fn record_sent(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        telegram_message_id: &str,
        telegram_file_id: Option<&str>,
        updated_at: &str,
    ) -> Result<(), SendStateError> {
        let changed = self
            .connection
            .execute(
                "UPDATE telegram_send_attempts \
                 SET state = 'SENT', telegram_message_id = ?3, telegram_file_id = ?4, \
                     last_error_code = NULL, last_error_message = NULL, updated_at = ?5 \
                 WHERE chat_id = ?1 AND idempotency_key = ?2",
                params![
                    chat_id,
                    idempotency_key,
                    telegram_message_id,
                    telegram_file_id,
                    updated_at
                ],
            )
            .map_err(send_state_error)?;
        if changed == 0 {
            return Err(SendStateError::NotFound);
        }
        Ok(())
    }

    fn record_failed(
        &self,
        chat_id: &str,
        idempotency_key: &str,
        error_code: Option<i64>,
        error_message: &str,
        updated_at: &str,
    ) -> Result<(), SendStateError> {
        let changed = self
            .connection
            .execute(
                "UPDATE telegram_send_attempts \
                 SET state = 'FAILED', last_error_code = ?3, last_error_message = ?4, updated_at = ?5 \
                 WHERE chat_id = ?1 AND idempotency_key = ?2",
                params![
                    chat_id,
                    idempotency_key,
                    error_code.map(|code| code.to_string()),
                    error_message,
                    updated_at
                ],
            )
            .map_err(send_state_error)?;
        if changed == 0 {
            return Err(SendStateError::NotFound);
        }
        Ok(())
    }

    fn list_unsent(&self) -> Result<Vec<PendingSendRecord>, SendStateError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT chat_id, idempotency_key, message_kind, state, attempt_count \
                 FROM telegram_send_attempts \
                 WHERE state IN ('PENDING', 'FAILED') \
                 ORDER BY updated_at ASC, id ASC",
            )
            .map_err(send_state_error)?;
        let rows = statement
            .query_map([], |row| {
                let state_value: String = row.get(3)?;
                let attempt_count: i64 = row.get(4)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    state_value,
                    attempt_count,
                ))
            })
            .map_err(send_state_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(send_state_error)?;
        rows.into_iter()
            .map(
                |(chat_id, idempotency_key, message_kind, state_value, attempt_count)| {
                    let state = SendState::parse(&state_value).ok_or_else(|| {
                        SendStateError::Store(format!("invalid send state: {state_value}"))
                    })?;
                    Ok(PendingSendRecord {
                        chat_id,
                        idempotency_key,
                        message_kind,
                        state,
                        attempt_count: attempt_count.clamp(0, i64::from(u32::MAX)) as u32,
                    })
                },
            )
            .collect()
    }
}
