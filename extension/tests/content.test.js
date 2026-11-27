import test from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync } from "node:fs";

// `URL` is a web platform API rather than an ECMAScript intrinsic, so a bare
// `vm` context does not provide it and `content-core.js` cannot parse links.
const context = { globalThis: {}, URL };
vm.runInNewContext(readFileSync(new URL("../src/content-core.js", import.meta.url), "utf8"), context);
const {
  articlesForMutation,
  archiveStatusToUiState,
  canonicalTweetUrl,
  createArchiveButton,
  createStatusPoller,
  createStatusQueryBatches,
  extractTweet,
  parseTweetId,
  parseTweetTarget,
  primaryTweetLink,
  setArchiveButtonState,
} = context.globalThis.XArchiveContent;

test("parses numeric Tweet IDs and canonical URLs", () => {
  assert.equal(parseTweetId("https://x.com/alice/status/123456789"), "123456789");
  assert.equal(parseTweetId("https://twitter.com/alice/statuses/42"), "42");
  assert.equal(canonicalTweetUrl("https://x.com/alice/status/123456789"), "https://x.com/i/status/123456789");
  assert.equal(parseTweetId("https://x.com/alice/likes"), null);
});

test("keeps the parsed host of accepted Tweet links", () => {
  assert.equal(canonicalTweetUrl("https://twitter.com/alice/status/42"), "https://twitter.com/i/status/42");
  assert.equal(canonicalTweetUrl("https://X.com/alice/status/42"), "https://x.com/i/status/42");
  // The value is built inside the `vm` realm, so compare a plain copy.
  assert.deepEqual(JSON.parse(JSON.stringify(parseTweetTarget("https://x.com/alice/status/42"))), { host: "x.com", tweetId: "42" });
});

test("rejects links whose host only looks like an X host", () => {
  // CodeQL js/incomplete-url-substring-sanitization: the allowed host name must
  // not be matched anywhere inside an unparsed URL string.
  assert.equal(parseTweetId("https://evil.example/twitter.com/status/123"), null);
  assert.equal(parseTweetId("https://evil.example/x.com/status/123"), null);
  assert.equal(canonicalTweetUrl("https://evil.example/twitter.com/status/123"), null);
  assert.equal(canonicalTweetUrl("https://x.com.evil.example/alice/status/123"), null);
  assert.equal(canonicalTweetUrl("https://twitter.com.evil.example/alice/status/123"), null);
  assert.equal(canonicalTweetUrl("https://evil-x.com/alice/status/123"), null);
  assert.equal(canonicalTweetUrl("https://notx.com/alice/status/123"), null);
  assert.equal(canonicalTweetUrl("https://x.com.attacker.net/alice/status/123"), null);
  assert.equal(canonicalTweetUrl("https://user@evil.example/alice/status/123"), null);
  // Credentials are rejected even when the host itself is allowlisted.
  assert.equal(canonicalTweetUrl("https://user:pw@x.com/alice/status/123"), null);
});

test("accepts a status link with trailing path segments", () => {
  // `/status/<id>/...` is a valid permanent-link shape; the ID is still the
  // last status segment matched, so trailing segments must not reject it.
  assert.equal(canonicalTweetUrl("https://x.com/alice/status/123/extra"), "https://x.com/i/status/123");
});

test("rejects unsupported schemes, credentials, ports and path shapes", () => {
  assert.equal(canonicalTweetUrl("http://x.com/alice/status/123"), null);
  assert.equal(canonicalTweetUrl("javascript:alert(1)//x.com/alice/status/123"), null);
  assert.equal(canonicalTweetUrl("data:text/html,https://x.com/alice/status/123"), null);
  assert.equal(canonicalTweetUrl("https://alice@evil.example/x.com/status/123"), null);
  assert.equal(canonicalTweetUrl("https://x.com:8443/alice/status/123"), null);
  assert.equal(canonicalTweetUrl("https://x.com/alice/status/123abc"), null);
  assert.equal(canonicalTweetUrl("https://x.com/alice/status/"), null);
  assert.equal(canonicalTweetUrl("https://x.com/status/123"), null);
  assert.equal(canonicalTweetUrl("https://x.com/"), null);
  assert.equal(canonicalTweetUrl(""), null);
  assert.equal(canonicalTweetUrl(null), null);
  assert.equal(canonicalTweetUrl(undefined), null);
});

test("ignores a status path hidden in the query string or fragment", () => {
  assert.equal(canonicalTweetUrl("https://x.com/alice?next=https://evil.example/twitter.com/status/123"), null);
  assert.equal(canonicalTweetUrl("https://x.com/alice#https://evil.example/status/123"), null);
  assert.equal(canonicalTweetUrl("https://evil.example/?u=https://x.com/alice/status/123"), null);
});

test("accepts the /i/status/ shape used by canonical links", () => {
  assert.equal(canonicalTweetUrl("https://x.com/i/status/123"), "https://x.com/i/status/123");
  assert.equal(canonicalTweetUrl("https://x.com/i/statuses/123"), "https://x.com/i/status/123");
  assert.equal(parseTweetId("https://x.com/i/web/status/123"), "123");
});

test("resolves relative hrefs against the page base only", () => {
  assert.equal(canonicalTweetUrl("/alice/status/123", "https://x.com/home"), "https://x.com/i/status/123");
  assert.equal(canonicalTweetUrl("/alice/status/123", "https://twitter.com/home"), "https://twitter.com/i/status/123");
  assert.equal(canonicalTweetUrl("/alice/status/123", "https://evil.example/home"), null);
  assert.equal(canonicalTweetUrl("//evil.example/alice/status/123", "https://x.com/home"), null);
});

test("extracts the stable Tweet metadata fields", () => {
  const textNode = { textContent: "hello  world" };
  const timeNode = { dateTime: "2026-09-08T10:00:00.000Z" };
  const link = { href: "https://x.com/alice/status/123" };
  const nodes = new Map([
    ['a[href*="/status/"]', link],
    ['[data-testid="User-Name"] a[href^="/"]', { textContent: "alice" }],
    ['[data-testid="User-Name"]', { textContent: "Alice" }],
    ['[data-testid="tweetText"]', textNode],
    ["time", timeNode],
    ['[data-testid="socialContext"]', null],
  ]);
  const article = {
    querySelector(selector) {
      return nodes.get(selector) || null;
    },
  };
  assert.deepEqual(JSON.parse(JSON.stringify(extractTweet(article))), {
    tweet_id: "123",
    url: "https://x.com/i/status/123",
    username: "alice",
    display_name: "Alice",
    text: "hello world",
    created_at: "2026-09-08T10:00:00.000Z",
    tweet_type: "post",
    reply_to: null,
    quoted_tweet: null,
  });
});

test("extracts nested quoted tweet cards", () => {
  const textNode = { textContent: "quoting" };
  const quoteTextNode = { textContent: "original post" };
  const quoteTimeNode = { dateTime: "2026-09-08T09:00:00.000Z" };
  const timeNode = { dateTime: "2026-09-08T10:00:00.000Z" };
  const link = { href: "https://x.com/alice/status/123" };
  const quoteLink = { href: "https://x.com/bob/status/987" };
  let quoteCardQueried = false;
  const quoteCard = {
    querySelector(selector) {
      quoteCardQueried = true;
      if (selector === '[data-testid="User-Name"] a[href^="/"]') return { textContent: "bob" };
      if (selector === '[data-testid="User-Name"]') return { textContent: "Bob" };
      if (selector === '[data-testid="tweetText"]') return quoteTextNode;
      if (selector === "time") return quoteTimeNode;
      return null;
    },
  };
  quoteLink.closest = () => quoteCard;
  const nodes = new Map([
    ['a[href*="/status/"]', link],
    ['[data-testid="User-Name"] a[href^="/"]', { textContent: "alice" }],
    ['[data-testid="User-Name"]', { textContent: "Alice" }],
    ['[data-testid="tweetText"]', textNode],
    ["time", timeNode],
    ['[data-testid="socialContext"]', null],
    ['div[role="link"] a[href*="/status/"]', quoteLink],
  ]);
  const article = {
    querySelector(selector) {
      return nodes.get(selector) || null;
    },
  };
  const tweet = JSON.parse(JSON.stringify(extractTweet(article)));
  assert.equal(tweet.tweet_type, "quote");
  assert.ok(quoteCardQueried);
  assert.deepEqual(tweet.quoted_tweet, {
    tweet_id: "987",
    url: "https://x.com/i/status/987",
    username: "bob",
    display_name: "Bob",
    text: "original post",
    created_at: "2026-09-08T09:00:00.000Z",
    tweet_type: "post",
    reply_to: null,
    quoted_tweet: null,
  });
});

test("selects the permalink instead of reply context or quoted status links", () => {
  const replyLink = { href: "https://x.com/carol/status/111" };
  const permalink = { href: "https://x.com/alice/status/123" };
  const quoteLink = { href: "https://x.com/bob/status/987" };
  const timeNode = { dateTime: "2026-09-08T10:00:00.000Z", closest: () => permalink };
  quoteLink.closest = () => ({ querySelector() { return null; } });
  const nodes = new Map([
    ["time", timeNode],
    ['[data-testid="tweetText"]', { textContent: "replying with a quote" }],
    ['[data-testid="socialContext"]', { textContent: "Replying to @carol" }],
    ['[data-testid="User-Name"] a[href^="/"]', { textContent: "alice" }],
    ['[data-testid="User-Name"]', { textContent: "Alice" }],
    ['div[role="link"] a[href*="/status/"]', quoteLink],
  ]);
  const article = {
    querySelector(selector) {
      return nodes.get(selector) || null;
    },
    querySelectorAll(selector) {
      if (selector === 'a[href*="/status/"]') return [replyLink, permalink, quoteLink];
      return [];
    },
  };

  assert.equal(primaryTweetLink(article), permalink);
  assert.deepEqual(JSON.parse(JSON.stringify(extractTweet(article))), {
    tweet_id: "123",
    url: "https://x.com/i/status/123",
    username: "alice",
    display_name: "Alice",
    text: "replying with a quote",
    created_at: "2026-09-08T10:00:00.000Z",
    tweet_type: "quote",
    reply_to: "111",
    quoted_tweet: {
      tweet_id: "987",
      url: "https://x.com/i/status/987",
      username: null,
      display_name: null,
      text: null,
      created_at: null,
      tweet_type: "post",
      reply_to: null,
      quoted_tweet: null,
    },
  });
});

test("does not report the current Tweet as its own reply parent", () => {
  const permalink = { href: "https://x.com/alice/status/123" };
  const article = {
    querySelector(selector) {
      if (selector === "time") return { dateTime: "2026-09-08T10:00:00.000Z", closest: () => permalink };
      if (selector === '[data-testid="tweetText"]') return { textContent: "reply without parent link" };
      if (selector === '[data-testid="socialContext"]') return { textContent: "Replying to @unknown" };
      return null;
    },
    querySelectorAll(selector) {
      return selector === 'a[href*="/status/"]' ? [permalink] : [];
    },
  };

  assert.equal(extractTweet(article).tweet_type, "reply");
  assert.equal(extractTweet(article).reply_to, null);
});

test("returns only articles affected by an added mutation node", () => {
  const article = {
    matches: (selector) => selector.includes("article"),
    closest: () => null,
    querySelector: (selector) => {
      if (selector === "time") return { dateTime: "2026-09-08T10:00:00.000Z", closest: () => ({ href: "https://x.com/alice/status/123" }) };
      if (selector === '[data-testid="tweetText"]') return { textContent: "new tweet" };
      return null;
    },
    querySelectorAll: (selector) => selector === 'a[href*="/status/"]' ? [{ href: "https://x.com/alice/status/123" }] : [],
  };
  const unrelated = {
    matches: () => false,
    querySelectorAll: () => [],
  };

  const affected = articlesForMutation({}, { addedNodes: [article, unrelated] });
  assert.equal(affected.length, 1);
  assert.equal(affected[0], article);
});

test("deduplicates and batches status queries at the Browser protocol limit", () => {
  const batches = createStatusQueryBatches(["1", 1, "2", "invalid", "3"], 2);
  assert.equal(batches.length, 2);
  assert.equal(batches[0].join(","), "1,2");
  assert.equal(batches[1].join(","), "3");
});

test("maps archive statuses to stable button states", () => {
  assert.equal(archiveStatusToUiState({ state: "NOT_ARCHIVED" }), "idle");
  assert.equal(archiveStatusToUiState({ state: "QUEUED" }), "queued");
  assert.equal(archiveStatusToUiState({ state: "DOWNLOADING" }), "running");
  assert.equal(archiveStatusToUiState({ state: "COMPLETE" }), "complete");
  assert.equal(archiveStatusToUiState({ state: "AUTH_REQUIRED" }), "auth_required");
  assert.equal(archiveStatusToUiState({ state: "FAILED" }), "failed");
});

test("updates button state and accessibility attributes", () => {
  const attributes = new Map();
  const button = {
    dataset: {},
    setAttribute(name, value) { attributes.set(name, value); },
  };
  setArchiveButtonState(button, "running");
  assert.equal(button.dataset.xarchiveState, "running");
  assert.equal(button.disabled, true);
  assert.equal(attributes.get("aria-busy"), "true");
  assert.equal(attributes.get("aria-label"), "XArchive：归档中");
});

test("createArchiveButton uses desktop neutral 7px idle style and reserves #2e6f59 for complete", () => {
  const attributes = new Map();
  const appended = [];
  const makeNode = () => ({
    type: "",
    dataset: {},
    style: { cssText: "" },
    children: [],
    setAttribute(name, value) { attributes.set(name, value); },
    append(...nodes) { this.children.push(...nodes); },
    querySelector(selector) {
      if (selector === "[data-xarchive-label]") return this.children[2] || null;
      if (selector === "[data-xarchive-dot]") return this.children[1] || null;
      return null;
    },
    addEventListener() {},
  });
  let createdButton = null;
  const articleAttrs = new Map();
  const article = {
    hasAttribute: () => false,
    setAttribute(name, value) { articleAttrs.set(name, value); },
    querySelector(selector) {
      if (selector === 'a[href*="/status/"]') {
        return { getAttribute: () => "https://x.com/alice/status/2125606699356635170" };
      }
      if (selector === '[role="group"]') {
        return { append(node) { appended.push(node); } };
      }
      return null;
    },
    querySelectorAll: () => [],
    textContent: "fixture tweet",
  };
  const doc = {
    createElement: (tag) => {
      const node = makeNode();
      if (tag === "button" && !createdButton) createdButton = node;
      return node;
    },
  };
  const button = createArchiveButton(article, () => {}, doc);
  assert.equal(button, createdButton);
  assert.equal(appended.length, 1);
  assert.match(createdButton.style.cssText, /border-radius:7px/);
  assert.doesNotMatch(createdButton.style.cssText, /999px|#2e6f59|#1b2a1b|#c6f277/);
  assert.notEqual(createdButton.style.color, "#2e6f59");
  setArchiveButtonState(createdButton, "queued");
  assert.equal(createdButton.style.color, "#9a5b00");
  assert.equal(createdButton.children[1].style.display, "inline-block");
  assert.equal(createdButton.children[2].textContent, "排队中");
  setArchiveButtonState(createdButton, "complete");
  assert.equal(createdButton.style.color, "#2e6f59");
  assert.equal(createdButton.children[2].textContent, "已归档");
});

test("createStatusPoller polls queued/running tweets until terminal and respects hidden/stop", async () => {
  const scheduled = [];
  let nextId = 1;
  let hidden = false;
  const states = ["running", "complete"];
  const article = { id: "art-1" };
  let poller;
  poller = createStatusPoller({
    intervalMs: 100,
    maxAttempts: 5,
    isDocumentHidden: () => hidden,
    setTimer: (fn) => {
      const id = nextId++;
      scheduled.push({ id, fn });
      return id;
    },
    clearTimer: (id) => {
      const idx = scheduled.findIndex((item) => item.id === id);
      if (idx >= 0) scheduled.splice(idx, 1);
    },
    queryStatuses: async (articles) => {
      assert.equal(articles.length, 1);
      assert.equal(articles[0], article);
      const next = states.shift();
      poller.track("2125853813256986653", article, next);
    },
  });

  poller.track("2125853813256986653", article, "queued");
  assert.equal(poller.pendingCount(), 1);
  assert.equal(scheduled.length, 1);

  hidden = true;
  poller.onVisibilityChange();
  assert.equal(scheduled.length, 0);

  hidden = false;
  poller.onVisibilityChange();
  assert.equal(scheduled.length, 1);

  await scheduled.shift().fn();
  assert.equal(poller.pendingCount(), 1);
  assert.equal(scheduled.length, 1);

  await scheduled.shift().fn();
  assert.equal(poller.pendingCount(), 0);
  assert.equal(scheduled.length, 0);

  poller.track("2125853813256986653", article, "queued");
  assert.equal(poller.pendingCount(), 1);
  poller.stop();
  assert.equal(poller.pendingCount(), 0);
  assert.equal(scheduled.length, 0);
});