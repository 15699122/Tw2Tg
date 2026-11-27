(function () {
  if (typeof document === "undefined" || typeof MutationObserver === "undefined" || !globalThis.XArchiveContent) return;

  const { applyArchiveStatus, createStatusPoller, createStatusQueryBatches, setArchiveButtonState } = globalThis.XArchiveContent;
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
    if (!button) return null;
    return applyArchiveStatus(button, status);
  }

  const poller = createStatusPoller({
    isDocumentHidden: () => Boolean(document.hidden),
    queryStatuses: (articles) => queryStatuses(articles, { showChecking: false }),
  });

  function queryStatuses(articles, options = {}) {
    const showChecking = options.showChecking !== false;
    const articleById = new Map();
    for (const article of articles || []) {
      const tweetId = article?.getAttribute?.("data-xarchive-archive");
      const button = buttonForArticle(article);
      if (!tweetId || !button || inFlightStatusIds.has(tweetId)) continue;
      articleById.set(tweetId, article);
      if (showChecking) setArchiveButtonState(button, "checking");
    }
    const requests = createStatusQueryBatches([...articleById.keys()]).map((batch) => {
      batch.forEach((tweetId) => inFlightStatusIds.add(tweetId));
      return sendRuntimeMessage({
        type: "query_status",
        request_id: `status-${Date.now()}-${batch[0]}`,
        tweet_ids: batch,
      })
        .then((response) => {
          for (const status of response?.statuses || []) {
            const tweetId = String(status.tweet_id);
            const article = articleById.get(tweetId);
            if (!article) continue;
            const nextState = updateArticleButton(article, status);
            if (nextState) poller.track(tweetId, article, nextState);
          }
        })
        .catch(() => {
          poller.stop();
          for (const tweetId of batch) {
            const article = articleById.get(tweetId);
            const button = buttonForArticle(article);
            if (button) setArchiveButtonState(button, "disconnected");
          }
        })
        .finally(() => batch.forEach((tweetId) => inFlightStatusIds.delete(tweetId)));
    });
    return Promise.all(requests);
  }

  document.addEventListener?.("visibilitychange", () => poller.onVisibilityChange());

  globalThis.XArchiveContent.startContentScript({
    onArchive: (tweet, button) => {
      setArchiveButtonState(button, "submitting");
      const requestId = `archive-${tweet.tweet_id}-${Date.now()}`;
      sendRuntimeMessage({ type: "archive_request", request_id: requestId, tweet })
        .then((response) => {
          const nextState = applyArchiveStatus(button, response);
          const article = button?.closest?.("article") || document.querySelector?.(`article[data-xarchive-archive="${tweet.tweet_id}"]`);
          if (article) poller.track(tweet.tweet_id, article, nextState);
        })
        .catch(() => {
          poller.stop();
          setArchiveButtonState(button, "disconnected");
        });
    },
    onArticles: (articles) => queryStatuses(articles, { showChecking: true }),
  });
})();