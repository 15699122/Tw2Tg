(function (global) {
  const BUTTON_ATTRIBUTE = "data-xarchive-archive";
  const TWEET_LINK_SELECTOR = 'a[href*="/status/"]';

  function parseTweetId(url) {
    const match = String(url).match(/\/(?:status|statuses)\/(\d+)/);
    return match ? match[1] : null;
  }

  function canonicalTweetUrl(url) {
    const tweetId = parseTweetId(url);
    if (!tweetId) return null;
    const host = String(url).includes("twitter.com") ? "twitter.com" : "x.com";
    return `https://${host}/i/status/${tweetId}`;
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
    canonicalTweetUrl,
    extractTweet,
    findTweetArticles,
    createArchiveButton,
    installArchiveButtons,
    startContentScript,
  };
})(globalThis);