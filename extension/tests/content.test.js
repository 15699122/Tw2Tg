import test from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync } from "node:fs";

// `URL` is a web platform API rather than an ECMAScript intrinsic, so a bare
// `vm` context does not provide it and `content-core.js` cannot parse links.
const context = { globalThis: {}, URL };
vm.runInNewContext(readFileSync(new URL("../src/content-core.js", import.meta.url), "utf8"), context);
const { canonicalTweetUrl, extractTweet, parseTweetId } = context.globalThis.XArchiveContent;

test("parses numeric Tweet IDs and canonical URLs", () => {
  assert.equal(parseTweetId("https://x.com/alice/status/123456789"), "123456789");
  assert.equal(parseTweetId("https://twitter.com/alice/statuses/42"), "42");
  assert.equal(canonicalTweetUrl("https://x.com/alice/status/123456789"), "https://x.com/i/status/123456789");
  assert.equal(canonicalTweetUrl("https://twitter.com/alice/statuses/42"), "https://twitter.com/i/status/42");
  assert.equal(parseTweetId("https://x.com/alice/likes"), null);
});

test("rejects hosts that only contain an allowed host as a substring", () => {
  // A substring test on the raw URL would accept every link below. The host
  // allowlist is checked on the parsed hostname instead.
  for (const href of [
    "https://evil.example/twitter.com/status/1",
    "https://x.com.evil.example/alice/status/1",
    "https://evil.example/x.com/alice/status/1",
    "https://notx.com/alice/status/1",
    "https://x.com.evil.example/",
  ]) {
    assert.equal(parseTweetId(href), null, `expected null for ${href}`);
    assert.equal(canonicalTweetUrl(href), null, `expected null for ${href}`);
  }
});

test("rejects non-https, credentialed and ported status links", () => {
  for (const href of [
    "http://x.com/alice/status/1",
    "https://user:pw@x.com/alice/status/1",
    "https://x.com:8443/alice/status/1",
  ]) {
    assert.equal(parseTweetId(href), null, `expected null for ${href}`);
  }
});

test("ignores host-like text in query and fragment", () => {
  // Query and fragment components are not part of the origin or the path, so
  // an allowlisted host mentioned there must not make a foreign link valid.
  assert.equal(parseTweetId("https://evil.example/?next=https://x.com/alice/status/1"), null);
  assert.equal(parseTweetId("https://evil.example/#https://x.com/alice/status/1"), null);
  // A trailing path segment after the ID is a real X permalink shape and must
  // still resolve to the Tweet itself.
  assert.equal(parseTweetId("https://x.com/alice/status/1/extra"), "1");
});

test("resolves relative status hrefs against the page, not a foreign origin", () => {
  assert.equal(parseTweetId("/alice/status/7", "https://x.com/home"), "7");
  assert.equal(
    canonicalTweetUrl("/alice/status/7", "https://twitter.com/home"),
    "https://twitter.com/i/status/7",
  );
  // Without a page base a relative href has no trustworthy origin.
  assert.equal(parseTweetId("/alice/status/7"), null);
  // A relative href must never be attributed to a page on another host.
  assert.equal(parseTweetId("/alice/status/7", "https://evil.example/home"), null);
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