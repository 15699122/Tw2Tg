import test from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync } from "node:fs";

const context = { globalThis: {} };
vm.runInNewContext(readFileSync(new URL("../src/content-core.js", import.meta.url), "utf8"), context);
const { canonicalTweetUrl, extractTweet, parseTweetId } = context.globalThis.XArchiveContent;

test("parses numeric Tweet IDs and canonical URLs", () => {
  assert.equal(parseTweetId("https://x.com/alice/status/123456789"), "123456789");
  assert.equal(parseTweetId("https://twitter.com/alice/statuses/42"), "42");
  assert.equal(canonicalTweetUrl("https://x.com/alice/status/123456789"), "https://x.com/i/status/123456789");
  assert.equal(parseTweetId("https://x.com/alice/likes"), null);
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