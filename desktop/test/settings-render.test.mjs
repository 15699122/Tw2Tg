import test from "node:test";
import assert from "node:assert/strict";
import { build } from "esbuild";
import { fileURLToPath } from "node:url";

test("settings render with the system proxy summary and editable diagnosis URL", async () => {
  const result = await build({
    stdin: { contents: `
      import React from 'react';
      import { renderToStaticMarkup } from 'react-dom/server';
      import SettingsPage from './src/pages/settings-page.jsx';
      export const html = renderToStaticMarkup(React.createElement(SettingsPage, {
        status: {}, errors: {}, extension: {}, expandedSections: {proxy: true, transfer: true},
        galleryDlMessage: '', proxySettings: {proxy_mode: 'system'},
        proxySystem: {backend: 'windows-os', static_proxies: [], bypass: [], child_coverage: 'inherits-environment'},
        proxyDiagnoseUrl: 'https://fixture.example/path', setProxyDiagnoseUrl: () => {},
      }));`, resolveDir: fileURLToPath(new URL('..', import.meta.url)), loader: 'jsx' },
    bundle: true, platform: 'node', format: 'cjs', jsx: 'automatic', write: false,
  });
  const mod = { exports: {} };
  new Function('module', 'exports', 'require', result.outputFiles[0].text)(mod, mod.exports, (await import('node:module')).createRequire(import.meta.url));
  assert.match(mod.exports.html, /https:\/\/fixture\.example\/path/);
  assert.match(mod.exports.html, /Windows 系统代理/);
  assert.match(mod.exports.html, /使用 aria2 传输媒体/);
});
