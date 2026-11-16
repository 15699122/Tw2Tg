export function acceptedRecordSelection(preview, selectedIndices) {
  return preview.entries.flatMap((entry, index) =>
    entry.verdict === "accept" && (!selectedIndices || selectedIndices.includes(index)) ? [{ index }] : [],
  );
}

export function recordImportErrorMessage(reason) {
  const message = String(reason);
  if (message === "record import cancelled while waiting for workspace maintenance") {
    return "已取消等待；记录未导入。";
  }
  return `接受失败：${message}`;
}
