import test from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync } from "node:fs";

// `URL` is a web platform API rather than an ECMAScript intrinsic, so a bare
// `vm` context does not provide it and `content-core.js` cannot parse links.
const context = { globalThis: {}, URL };
vm.runInNewContext(readFileSync(new URL("../src/content-core.js", import.meta.url), "utf8"), context);
const { canonicalTweetUrl, extractTweet, parseTweetId, parseTweetTarget } = context.globalThis.XArchiveContent;

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