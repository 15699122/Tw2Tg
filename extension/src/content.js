(function () {
  if (typeof document === "undefined" || typeof MutationObserver === "undefined" || !globalThis.XArchiveContent) return;
  globalThis.XArchiveContent.startContentScript({
    onArchive: (tweet, button) => {
      button.disabled = true;
      button.textContent = "已提交";
      const requestId = `archive-${tweet.tweet_id}-${Date.now()}`;
      if (typeof chrome !== "undefined" && chrome.runtime?.sendMessage) {
        chrome.runtime.sendMessage({ type: "archive_request", request_id: requestId, tweet }, (response) => {
          if (chrome.runtime.lastError || response?.message_type === "error") {
            button.disabled = false;
            button.textContent = "重试";
          }
        });
      } else {
        window.dispatchEvent(new CustomEvent("xarchive:archive", { detail: tweet }));
      }
    },
  });
})();