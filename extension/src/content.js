(function () {
  if (typeof document === "undefined" || typeof MutationObserver === "undefined" || !globalThis.XArchiveContent) return;

  const { applyArchiveStatus, createStatusQueryBatches, setArchiveButtonState } = globalThis.XArchiveContent;
  const inFlightStatusIds = new Set();

  function buttonForArticle(article) {
    return article?.querySelector?.("button[data-xarchive-archive]") || null;
  }

  function sendRuntimeMessage(message) {
    return new Promise((resolve, reject) => {
      if (typeof chrome === "undefined" || !chrome.runtime?.sendMessage) {
        reject(Object.assign(new Error("Native Messaging is unavailable"), { code: "NATIVE_HOST_UNAVAILABLE", retryable: true }));
        return;
      }
      chrome.runtime.sendMessage(message, (response) => {
        const runtimeError = chrome.runtime.lastError;
        if (runtimeError) {
          reject(Object.assign(new Error(runtimeError.message), { code: "NATIVE_HOST_ERROR", retryable: true }));
          return;
        }
        if (response?.message_type === "error") {
          reject(Object.assign(new Error(response.error_message || "Native Host error"), {
            code: response.error_code || "NATIVE_HOST_ERROR",
            retryable: response.retryable !== false,
          }));
          return;
        }
        resolve(response);
      });
    });
  }

  function updateArticleButton(article, status) {
    const button = buttonForArticle(article);
    if (button) applyArchiveStatus(button, status);
  }

  function queryStatuses(articles) {
    const articleById = new Map();
    for (const article of articles || []) {
      const tweetId = article?.getAttribute?.("data-xarchive-archive");
      const button = buttonForArticle(article);
      if (!tweetId || !button || inFlightStatusIds.has(tweetId)) continue;
      articleById.set(tweetId, article);
      setArchiveButtonState(button, "checking");
    }
    for (const batch of createStatusQueryBatches([...articleById.keys()])) {
      batch.forEach((tweetId) => inFlightStatusIds.add(tweetId));
      sendRuntimeMessage({
        type: "query_status",
        request_id: `status-${Date.now()}-${batch[0]}`,
        tweet_ids: batch,
      })
        .then((response) => {
          for (const status of response?.statuses || []) {
            const article = articleById.get(String(status.tweet_id));
            if (article) updateArticleButton(article, status);
          }
        })
        .catch(() => {
          for (const tweetId of batch) {
            const article = articleById.get(tweetId);
            const button = buttonForArticle(article);
            if (button) setArchiveButtonState(button, "disconnected");
          }
        })
        .finally(() => batch.forEach((tweetId) => inFlightStatusIds.delete(tweetId)));
    }
  }

  globalThis.XArchiveContent.startContentScript({
    onArchive: (tweet, button) => {
      setArchiveButtonState(button, "submitting");
      const requestId = `archive-${tweet.tweet_id}-${Date.now()}`;
      sendRuntimeMessage({ type: "archive_request", request_id: requestId, tweet })
        .then((response) => applyArchiveStatus(button, response))
        .catch(() => setArchiveButtonState(button, "disconnected"));
    },
    onArticles: queryStatuses,
  });
})();