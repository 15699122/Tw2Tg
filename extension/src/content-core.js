(function (global) {
  const BUTTON_ATTRIBUTE = "data-xarchive-archive";
  const TWEET_LINK_SELECTOR = 'a[href*="/status/"]';
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

  function extractQuotedTweet(article) {
    const quoteLink = article?.querySelector?.('div[role="link"] a[href*="/status/"]');
    if (!quoteLink) return null;
    const href = quoteLink.href || quoteLink.getAttribute?.("href") || "";
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
    const link = article?.querySelector?.(TWEET_LINK_SELECTOR);
    const href = link?.href || link?.getAttribute?.("href") || "";
    const tweetId = parseTweetId(href);
    if (!tweetId) return null;
    const time = article.querySelector?.("time");
    const text = firstText(article, ['[data-testid="tweetText"]']);
    const socialContext = textContent(article.querySelector?.('[data-testid="socialContext"]'));
    const replyTo = article.querySelector?.('a[href*="/status/"]')?.href;
    const isReply = /reply/i.test(socialContext || "");
    const isQuote = Boolean(article.querySelector?.('div[role="link"] a[href*="/status/"]'));
    return {
      tweet_id: tweetId,
      url: canonicalTweetUrl(href),
      username: firstText(article, ['[data-testid="User-Name"] a[href^="/"]']),
      display_name: firstText(article, ['[data-testid="User-Name"]']),
      text: text || "",
      created_at: time?.dateTime || null,
      tweet_type: isQuote ? "quote" : isReply ? "reply" : "post",
      reply_to: isReply && replyTo ? parseTweetId(replyTo) : null,
      quoted_tweet: isQuote ? extractQuotedTweet(article) : null,
    };
  }

  function findTweetArticles(root) {
    root = root || document;
    return [...root.querySelectorAll('article[data-testid="tweet"], article')].filter((article) => extractTweet(article));
  }

  function createArchiveButton(article, onArchive, documentRef) {
    documentRef = documentRef || document;
    if (!article || article.hasAttribute?.(BUTTON_ATTRIBUTE)) return null;
    const tweet = extractTweet(article);
    if (!tweet) return null;
    const button = documentRef.createElement("button");
    button.type = "button";
    button.setAttribute(BUTTON_ATTRIBUTE, tweet.tweet_id);
    button.setAttribute("aria-label", "Archive Tweet with XArchive");
    button.textContent = "保存";
    button.style.cssText = "margin-left:8px;border:1px solid #53684b;border-radius:999px;padding:4px 9px;color:#c6f277;background:#1b2a1b;cursor:pointer;font:11px sans-serif";
    button.addEventListener("click", () => onArchive(tweet, button));
    const toolbar = article.querySelector?.('[role="group"]') || article;
    toolbar.append(button);
    article.setAttribute(BUTTON_ATTRIBUTE, tweet.tweet_id);
    return button;
  }

  function installArchiveButtons(root, onArchive) {
    root = root || document;
    onArchive = onArchive || function () {};
    let count = 0;
    for (const article of findTweetArticles(root)) {
      if (createArchiveButton(article, onArchive, root.ownerDocument || document)) count += 1;
    }
    return count;
  }

  function startContentScript(options) {
    options = options || {};
    const root = options.root || document;
    installArchiveButtons(root, options.onArchive);
    const Observer = options.observerClass || MutationObserver;
    const observer = new Observer(() => installArchiveButtons(root, options.onArchive));
    observer.observe(root.body || root, { childList: true, subtree: true });
    return () => observer.disconnect();
  }

  global.XArchiveContent = {
    parseTweetId,
    parseTweetTarget,
    canonicalTweetUrl,
    extractTweet,
    findTweetArticles,
    createArchiveButton,
    installArchiveButtons,
    startContentScript,
  };
})(globalThis);