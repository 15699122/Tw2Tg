(function (global) {
  const BUTTON_ATTRIBUTE = "data-xarchive-archive";
  const TWEET_LINK_SELECTOR = 'a[href*="/status/"]';
  const QUOTE_LINK_SELECTOR = 'div[role="link"] a[href*="/status/"]';
  const ARTICLE_SELECTOR = 'article[data-testid="tweet"], article';
  const STATUS_BATCH_SIZE = 100;
  // Tweet identity may only come from these exact hosts. The allowlist has to be
  // checked on the parsed URL host: a substring test on the raw string is also
  // satisfied by a path, query, or fragment component such as
  // `https://evil.example/twitter.com/status/1` or `https://x.com.evil.example/`.
  const TWEET_HOSTS = new Set(["x.com", "twitter.com"]);
  const TWEET_STATUS_SEGMENTS = new Set(["status", "statuses"]);
  const TWEET_ID_PATTERN = /^\d+$/;

  function pageBaseUrl() {
    return typeof document !== "undefined" ? document?.baseURI : undefined;
  }

  // Parse one href into a validated host and Tweet ID, or return null when the
  // link is not an https X/Twitter status link. Relative hrefs resolve against
  // the current page only, so a link cannot be attributed to a foreign origin.
  function parseTweetTarget(href, base) {
    if (typeof href !== "string" || !href) return null;
    let parsed;
    try {
      parsed = new URL(href, base || pageBaseUrl());
    } catch {
      return null;
    }
    if (parsed.protocol !== "https:") return null;
    // `xarchive-protocol`'s `extract_tweet_id` compares the whole authority, so
    // embedded credentials, a port, or a subdomain must not be accepted here
    // either: the Extension would otherwise emit a URL the Desktop rejects.
    if (parsed.username || parsed.password || parsed.port) return null;
    const host = parsed.hostname.toLowerCase();
    if (!TWEET_HOSTS.has(host)) return null;
    const segments = parsed.pathname.split("/").filter(Boolean);
    for (let index = 1; index < segments.length - 1; index += 1) {
      if (!TWEET_STATUS_SEGMENTS.has(segments[index])) continue;
      const tweetId = segments[index + 1];
      if (TWEET_ID_PATTERN.test(tweetId)) return { host, tweetId };
    }
    return null;
  }

  function parseTweetId(url, base) {
    return parseTweetTarget(url, base)?.tweetId ?? null;
  }

  function canonicalTweetUrl(url, base) {
    const target = parseTweetTarget(url, base);
    if (!target) return null;
    return `https://${target.host}/i/status/${target.tweetId}`;
  }

  function textContent(node) {
    return node?.textContent?.replace(/\s+/g, " ").trim() || "";
  }

  function firstText(root, selectors) {
    for (const selector of selectors) {
      const value = textContent(root.querySelector?.(selector));
      if (value) return value;
    }
    return null;
  }

  function queryAll(root, selector) {
    if (!root?.querySelectorAll) return [];
    return [...root.querySelectorAll(selector)];
  }

  function linkHref(link) {
    return link?.href || link?.getAttribute?.("href") || "";
  }

  function quoteLinkFor(article) {
    const link = article?.querySelector?.(QUOTE_LINK_SELECTOR);
    return link && parseTweetId(linkHref(link)) ? link : null;
  }

  function statusLinksFor(article) {
    const links = queryAll(article, TWEET_LINK_SELECTOR);
    if (links.length) return links;
    const fallback = article?.querySelector?.(TWEET_LINK_SELECTOR);
    return fallback ? [fallback] : [];
  }

  function isQuoteLink(link, quoteLink) {
    if (!link || !quoteLink) return false;
    return link === quoteLink || parseTweetId(linkHref(link)) === parseTweetId(linkHref(quoteLink));
  }

  function primaryTweetLink(article) {
    const links = statusLinksFor(article);
    const quoteLink = quoteLinkFor(article);
    const nonQuoteLinks = links.filter((link) => !isQuoteLink(link, quoteLink));
    if (nonQuoteLinks.length === 1) return nonQuoteLinks[0];

    const time = article?.querySelector?.("time");
    const timeLink = time?.closest?.(TWEET_LINK_SELECTOR);
    if (timeLink && !isQuoteLink(timeLink, quoteLink)) return timeLink;
    return nonQuoteLinks[0] || links[0] || null;
  }

  function replyLinkFor(article, primaryLink, quoteLink) {
    return statusLinksFor(article).find((link) => {
      if (link === primaryLink || isQuoteLink(link, quoteLink)) return false;
      const id = parseTweetId(linkHref(link));
      return id && id !== parseTweetId(linkHref(primaryLink));
    }) || null;
  }

  function extractQuotedTweet(article) {
    const quoteLink = quoteLinkFor(article);
    if (!quoteLink) return null;
    const href = linkHref(quoteLink);
    const quotedTweetId = parseTweetId(href);
    if (!quotedTweetId) return null;
    const quoteCard = quoteLink.closest?.('div[role="link"]') || quoteLink.parentElement;
    return {
      tweet_id: quotedTweetId,
      url: canonicalTweetUrl(href),
      username: firstText(quoteCard, ['[data-testid="User-Name"] a[href^="/"]']),
      display_name: firstText(quoteCard, ['[data-testid="User-Name"]']),
      text: firstText(quoteCard, ['[data-testid="tweetText"]']),
      created_at: quoteCard?.querySelector?.("time")?.dateTime || null,
      tweet_type: "post",
      reply_to: null,
      quoted_tweet: null,
    };
  }

  function extractTweet(article) {
    const link = primaryTweetLink(article);
    const href = linkHref(link);
    const tweetId = parseTweetId(href);
    if (!tweetId) return null;
    const time = article.querySelector?.("time");
    const text = firstText(article, ['[data-testid="tweetText"]']);
    const socialContext = textContent(article.querySelector?.('[data-testid="socialContext"]'));
    const quoteLink = quoteLinkFor(article);
    const replyTo = replyLinkFor(article, link, quoteLink);
    const isReply = /reply/i.test(socialContext || "");
    const isQuote = Boolean(quoteLink && parseTweetId(linkHref(quoteLink)) !== tweetId);
    return {
      tweet_id: tweetId,
      url: canonicalTweetUrl(href),
      username: firstText(article, ['[data-testid="User-Name"] a[href^="/"]']),
      display_name: firstText(article, ['[data-testid="User-Name"]']),
      text: text || "",
      created_at: time?.dateTime || null,
      tweet_type: isQuote ? "quote" : isReply ? "reply" : "post",
      reply_to: isReply && replyTo ? parseTweetId(linkHref(replyTo)) : null,
      quoted_tweet: isQuote ? extractQuotedTweet(article) : null,
    };
  }

  function findTweetArticles(root) {
    root = root || document;
    return queryAll(root, ARTICLE_SELECTOR).filter((article) => extractTweet(article));
  }

  function articlesForMutation(root, mutation) {
    const articles = new Set();
    for (const node of mutation?.addedNodes || []) {
      if (node?.matches?.(`button[${BUTTON_ATTRIBUTE}]`)) continue;
      if (node?.matches?.(ARTICLE_SELECTOR)) articles.add(node);
      for (const article of findTweetArticles(node)) articles.add(article);
      const parent = node?.closest?.(ARTICLE_SELECTOR);
      if (parent) articles.add(parent);
    }
    return [...articles].filter((article) => extractTweet(article));
  }

  function installArchiveButtonsForArticles(articles, onArchive, documentRef) {
    let count = 0;
    for (const article of articles) {
      if (createArchiveButton(article, onArchive, documentRef)) count += 1;
    }
    return count;
  }

  function createStatusQueryBatches(tweetIds, batchSize = STATUS_BATCH_SIZE) {
    const uniqueIds = [...new Set((tweetIds || []).map(String).filter((id) => /^\d+$/.test(id)))];
    const batches = [];
    for (let index = 0; index < uniqueIds.length; index += batchSize) {
      batches.push(uniqueIds.slice(index, index + batchSize));
    }
    return batches;
  }

  function archiveStatusToUiState(status) {
    if (!status || status.state === "NOT_ARCHIVED") return "idle";
    if (status.state === "QUEUED") return "queued";
    if (["VALIDATING", "METADATA_READY", "DOWNLOADING", "DOWNLOADED", "TG_METADATA_SENDING", "TG_METADATA_SENT", "TG_MEDIA_UPLOADING"].includes(status.state)) return "running";
    if (status.state === "COMPLETE") return "complete";
    if (status.state === "AUTH_REQUIRED") return "auth_required";
    if (["FAILED", "CANCELLED", "INTERRUPTED"].includes(status.state)) return "failed";
    return "failed";
  }

  function applyArchiveButtonAppearance(button, state, hovered) {
    if (!button?.style) return;
    const dot = button.querySelector?.("[data-xarchive-dot]");
    if (state === "complete") {
      button.style.borderColor = "#2e6f59";
      button.style.color = "#2e6f59";
      button.style.background = "rgba(46,111,89,0.10)";
      if (dot?.style) {
        dot.style.display = "inline-block";
        dot.style.background = "#2e6f59";
      }
      return;
    }
    if (["checking", "submitting", "queued", "running"].includes(state)) {
      button.style.borderColor = "rgba(154,91,0,0.40)";
      button.style.color = "#9a5b00";
      button.style.background = "rgba(154,91,0,0.12)";
      if (dot?.style) {
        dot.style.display = "inline-block";
        dot.style.background = "#9a5b00";
      }
      return;
    }
    if (state === "failed" || state === "disconnected" || state === "auth_required") {
      button.style.borderColor = "rgba(180,35,24,0.35)";
      button.style.color = "#b42318";
      button.style.background = hovered ? "rgba(180,35,24,0.10)" : "rgba(180,35,24,0.05)";
      if (dot?.style) {
        dot.style.display = "inline-block";
        dot.style.background = "#b42318";
      }
      return;
    }
    button.style.borderColor = hovered ? "rgba(128,128,128,0.42)" : "rgba(128,128,128,0.28)";
    button.style.color = "inherit";
    button.style.background = hovered ? "rgba(128,128,128,0.14)" : "rgba(128,128,128,0.06)";
    if (dot?.style) dot.style.display = "none";
  }

  function setArchiveButtonState(button, state) {
    if (!button) return;
    const labels = {
      idle: "保存",
      checking: "检测中…",
      submitting: "提交中…",
      queued: "排队中",
      running: "归档中",
      complete: "已归档",
      auth_required: "需要登录",
      failed: "重试",
      disconnected: "重试",
    };
    const text = labels[state] || labels.failed;
    const disabled = ["checking", "submitting", "queued", "running", "complete"].includes(state);
    button.dataset.xarchiveState = state;
    button.disabled = disabled;
    button.setAttribute("aria-busy", String(["checking", "submitting", "running"].includes(state)));
    button.setAttribute("aria-label", `XArchive：${text}`);
    const labelNode = button.querySelector?.("[data-xarchive-label]");
    if (labelNode) {
      labelNode.textContent = text;
    } else {
      button.textContent = text;
    }
    applyArchiveButtonAppearance(button, state, false);
  }

  function applyArchiveStatus(button, status) {
    const state = archiveStatusToUiState(status);
    setArchiveButtonState(button, state);
    return state;
  }

  function createArchiveButton(article, onArchive, documentRef) {
    documentRef = documentRef || document;
    if (!article || article.hasAttribute?.(BUTTON_ATTRIBUTE)) return null;
    const tweet = extractTweet(article);
    if (!tweet) return null;
    const button = documentRef.createElement("button");
    button.type = "button";
    button.setAttribute(BUTTON_ATTRIBUTE, tweet.tweet_id);
    button.style.cssText = "margin:0 4px 0 8px;display:inline-flex;align-items:center;gap:5px;height:28px;padding:2px 9px;border:1px solid rgba(128,128,128,0.28);border-radius:7px;color:inherit;background:rgba(128,128,128,0.06);cursor:pointer;font:500 12px/1 -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;vertical-align:middle;transition:background-color .15s ease,border-color .15s ease";
    const icon = documentRef.createElement("span");
    icon.setAttribute("aria-hidden", "true");
    icon.style.cssText = "display:inline-flex;align-items:center";
    icon.innerHTML = '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 8v13H3V8"/><path d="M1 3h22v5H1z"/><path d="M10 12h4"/></svg>';
    const dot = documentRef.createElement("span");
    dot.setAttribute("data-xarchive-dot", "true");
    dot.setAttribute("aria-hidden", "true");
    dot.style.cssText = "display:none;width:6px;height:6px;border-radius:50%;flex:none";
    const label = documentRef.createElement("span");
    label.setAttribute("data-xarchive-label", "true");
    button.append?.(icon, dot, label);
    setArchiveButtonState(button, "idle");
    button.addEventListener("mouseenter", () => applyArchiveButtonAppearance(button, button.dataset.xarchiveState || "idle", true));
    button.addEventListener("mouseleave", () => applyArchiveButtonAppearance(button, button.dataset.xarchiveState || "idle", false));
    button.addEventListener("click", () => onArchive(tweet, button));
    const toolbar = article.querySelector?.('[role="group"]') || article;
    toolbar.append(button);
    article.setAttribute(BUTTON_ATTRIBUTE, tweet.tweet_id);
    return button;
  }

  function installArchiveButtons(root, onArchive) {
    root = root || document;
    onArchive = onArchive || function () {};
    return installArchiveButtonsForArticles(findTweetArticles(root), onArchive, root.ownerDocument || document);
  }

  function startContentScript(options) {
    options = options || {};
    const root = options.root || document;
    installArchiveButtons(root, options.onArchive);
    const Observer = options.observerClass || MutationObserver;
    const observer = new Observer((mutations) => {
      const articles = new Set();
      for (const mutation of mutations || []) {
        for (const article of articlesForMutation(root, mutation)) articles.add(article);
      }
      const affectedArticles = [...articles];
      installArchiveButtonsForArticles(affectedArticles, options.onArchive, root.ownerDocument || document);
      options.onArticles?.(affectedArticles);
    });
    observer.observe(root.body || root, { childList: true, subtree: true });
    options.onArticles?.(findTweetArticles(root));
    return () => observer.disconnect();
  }

  function createStatusPoller(options) {
    options = options || {};
    const queryStatuses = options.queryStatuses;
    const isDocumentHidden = options.isDocumentHidden || (() => false);
    const intervalMs = options.intervalMs ?? 1500;
    const maxAttempts = options.maxAttempts ?? 120;
    const setTimer = options.setTimer || setTimeout;
    const clearTimer = options.clearTimer || clearTimeout;

    const pending = new Map();
    let attempts = 0;
    let timerId = null;
    let inFlight = false;

    function clearScheduled() {
      if (timerId !== null) {
        clearTimer(timerId);
        timerId = null;
      }
    }

    function scheduleNext(delay) {
      if (timerId !== null || inFlight || pending.size === 0 || attempts >= maxAttempts || isDocumentHidden()) {
        return;
      }
      timerId = setTimer(() => {
        timerId = null;
        return pollOnce();
      }, delay ?? intervalMs);
    }

    async function pollOnce() {
      if (inFlight || pending.size === 0 || isDocumentHidden()) {
        return;
      }
      if (attempts >= maxAttempts) {
        clearScheduled();
        return;
      }
      attempts += 1;
      inFlight = true;
      try {
        await queryStatuses([...pending.values()]);
      } finally {
        inFlight = false;
        if (pending.size > 0 && attempts < maxAttempts && !isDocumentHidden()) {
          scheduleNext(intervalMs);
        }
      }
    }

    function track(tweetId, article, state) {
      const key = String(tweetId || "");
      if (!key) return;
      if (state === "queued" || state === "running") {
        if (article) pending.set(key, article);
        attempts = 0;
        scheduleNext(intervalMs);
      } else {
        pending.delete(key);
        if (pending.size === 0) {
          clearScheduled();
        }
      }
    }

    function onVisibilityChange() {
      if (isDocumentHidden()) {
        clearScheduled();
      } else if (pending.size > 0 && attempts < maxAttempts) {
        scheduleNext(0);
      }
    }

    function stop() {
      pending.clear();
      clearScheduled();
    }

    return {
      track,
      pollOnce,
      onVisibilityChange,
      stop,
      pendingCount: () => pending.size,
    };
  }

  global.XArchiveContent = {
    parseTweetId,
    parseTweetTarget,
    canonicalTweetUrl,
    primaryTweetLink,
    createStatusPoller,
    replyLinkFor,
    quoteLinkFor,
    articlesForMutation,
    createStatusQueryBatches,
    archiveStatusToUiState,
    setArchiveButtonState,
    applyArchiveStatus,
    extractTweet,
    findTweetArticles,
    createArchiveButton,
    installArchiveButtons,
    startContentScript,
  };
})(globalThis);