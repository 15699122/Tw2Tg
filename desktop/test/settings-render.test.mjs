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
      import DownloadConfigPage from './src/pages/download-config-page.jsx';
      import DownloadJobsPage from './src/pages/download-jobs-page.jsx';
      import StoragePage from './src/pages/storage-page.jsx';
      export const html = renderToStaticMarkup(React.createElement(SettingsPage, {
        status: {}, errors: {}, extension: {}, expandedSections: {proxy: true},
        galleryDlMessage: '', proxySettings: {proxy_mode: 'system'},
        proxySystem: {backend: 'windows-os', static_proxies: [], bypass: [], child_coverage: 'inherits-environment'},
        proxyDiagnoseUrl: 'https://fixture.example/path', setProxyDiagnoseUrl: () => {},
      }));
      export const configHtml = renderToStaticMarkup(React.createElement(DownloadConfigPage, {
        isWindows: true, errors: {}, expandedSections: {transfer: true},
        useAria2: true, aria2: {found: false},
      }));
      export const jobsHtml = renderToStaticMarkup(React.createElement(DownloadJobsPage, {}));
      export const storageHtml = renderToStaticMarkup(React.createElement(StoragePage, {}));`, resolveDir: fileURLToPath(new URL('..', import.meta.url)), loader: 'jsx' },
    bundle: true, platform: 'node', format: 'cjs', jsx: 'automatic', write: false,
  });
  const mod = { exports: {} };
  new Function('module', 'exports', 'require', result.outputFiles[0].text)(mod, mod.exports, (await import('node:module')).createRequire(import.meta.url));
  assert.match(mod.exports.html, /https:\/\/fixture\.example\/path/);
  assert.match(mod.exports.html, /Windows 系统代理/);
  // The diagnosis field must sit inside a `.settings-fields` container: the
  // label and input styling is scoped to that class, so a bare `.settings-field`
  // renders as an unstyled native control.
  assert.match(mod.exports.html, /settings-fields proxy-diagnose-fields/);
  // Download mode and aria2 no longer live on the settings page.
  assert.doesNotMatch(mod.exports.html, /使用 aria2 传输媒体/);
  assert.match(mod.exports.configHtml, /使用 aria2 传输媒体/);
  assert.match(mod.exports.configHtml, /下载配置/);
  // 拆分后的三个页面各自可渲染，且不互相携带对方的控件。
  assert.match(mod.exports.jobsHtml, /任务记录/);
  assert.doesNotMatch(mod.exports.jobsHtml, /使用 aria2 传输媒体/);
  assert.match(mod.exports.storageHtml, /存储位置/);
  assert.doesNotMatch(mod.exports.storageHtml, /aria2/);
});

test("download settings render the Windows aria2 manager or a non-Windows explanation", async () => {
  const result = await build({
    stdin: { contents: `
      import React from 'react';
      import { renderToStaticMarkup } from 'react-dom/server';
      import DownloadConfigPage from './src/pages/download-config-page.jsx';
      const props = {
        errors: {}, expandedSections: {transfer: true, aria2: true},
        useAria2: false, aria2: {found: false},
        outputSettings: {naming_mode: 'template', filename_template: '{username}_{tweet_id}', export_json: false, export_text: true},
        setOutputSettings: () => {}, saveOutputSettings: () => {},
      };
      export const windowsHtml = renderToStaticMarkup(React.createElement(DownloadConfigPage, { ...props, isWindows: true }));
      export const linuxHtml = renderToStaticMarkup(React.createElement(DownloadConfigPage, { ...props, isWindows: false }));`, resolveDir: fileURLToPath(new URL('..', import.meta.url)), loader: 'jsx' },
    bundle: true, platform: 'node', format: 'cjs', jsx: 'automatic', write: false,
  });
  const mod = { exports: {} };
  new Function('module', 'exports', 'require', result.outputFiles[0].text)(mod, mod.exports, (await import('node:module')).createRequire(import.meta.url));
  assert.match(mod.exports.windowsHtml, /下载并安装/);
  assert.match(mod.exports.windowsHtml, /使用 aria2 传输媒体/);
  // aria2 install management is now conditionally rendered only on Windows;
  // a non-Windows build simply omits the Aria2Settings block instead of
  // printing an explicit "not available" notice.
  assert.doesNotMatch(mod.exports.linuxHtml, /下载并安装/);
  assert.doesNotMatch(mod.exports.linuxHtml, /aria2-settings/);
  assert.doesNotMatch(mod.exports.linuxHtml, /自定义 aria2 路径/);
  assert.match(mod.exports.windowsHtml, /aria2-settings/);
  assert.match(mod.exports.windowsHtml, /output-settings/);
  assert.match(mod.exports.windowsHtml, /导出 tweet.json/);
  assert.match(mod.exports.windowsHtml, /独立恢复清单批次完成后生效/);
  assert.match(mod.exports.windowsHtml, /关闭 tweet\.json 和 tweet\.txt 导出后，内部恢复清单仍会保存 Tweet 与媒体的身份、路径、大小和校验值/);
  assert.match(mod.exports.windowsHtml, /不会保存 Tweet 正文、作者资料或回复\/引用关系/);
});

test("Toggle handles Enter once and preserves native Space and busy behavior", async () => {
  const result = await build({
    stdin: { contents: `export { Toggle } from './src/components/ui/toggle.jsx';`, resolveDir: fileURLToPath(new URL('..', import.meta.url)), loader: 'jsx' },
    bundle: true, platform: 'node', format: 'cjs', jsx: 'automatic', write: false,
  });
  const mod = { exports: {} };
  new Function('module', 'exports', 'require', result.outputFiles[0].text)(mod, mod.exports, (await import('node:module')).createRequire(import.meta.url));
  const calls = [];
  const input = (props) => mod.exports.Toggle({ checked: false, onCheckedChange: (value) => calls.push(value), ...props }).props.children[0].props;
  let prevented = 0;
  const enter = { key: 'Enter', repeat: false, preventDefault: () => prevented++ };
  input({}).onKeyDown(enter);
  assert.deepEqual(calls, [true]);
  input({checked: true}).onKeyDown(enter);
  assert.deepEqual(calls, [true, false]);
  input({}).onKeyDown({...enter, repeat: true});
  input({disabled: true}).onKeyDown(enter);
  input({}).onKeyDown({...enter, key: ' '});
  assert.deepEqual(calls, [true, false]);
  assert.equal(prevented, 4);
  input({}).onChange({target: {checked: true}});
  assert.deepEqual(calls, [true, false, true]);
});
