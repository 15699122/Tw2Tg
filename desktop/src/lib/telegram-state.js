export function telegramSettingsInput(settings) {
  const keys = ["enabled", "endpoint_mode", "api_base", "chat_id", "message_thread_id", "auto_send_on_archive", "credential_rotation_resume_policy", "upload_mode", "connect_timeout_seconds", "upload_processing_timeout_seconds"];
  return Object.fromEntries(keys.map((key) => [key, settings[key]]));
}

export function telegramActions(state) {
  return { cancel: state === "QUEUED" || state === "RETRY_WAIT", review: state === "UNKNOWN" };
}