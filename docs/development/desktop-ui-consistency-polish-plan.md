# Desktop UI Consistency and Polish Plan (2026-11-22)

Owner: Cross-platform Owner (shared CSS, shared page components, UI primitives in `desktop/src/`), Windows Platform Owner (Windows WebView2 GUI acceptance).
Status: `IMPLEMENTED_CROSS_PLATFORM` (2026-11-22, `CROSS_PLATFORM_CHANGE_REQUIRED` — UI presentation & component consistency only; does not alter Tauri IPC commands, storage schemas, or sidecar/download protocols). Linux/shared implementation and frontend checks are complete; Windows WebView2 GUI acceptance is pending (§6).
Round-2 follow-up (§7 findings) is `IMPLEMENTED_CROSS_PLATFORM` (2026-11-24) — see §8; Windows WebView2 acceptance for R1–R7 is `NOT_RUN`.

## 1. Objective

Align the Desktop UI implementation in `desktop/src/` with the repository's minimalist utility-first design system (`--surface` / `--surface-subtle` hierarchy, `1px` `--border`, `10px` `<Card>` containers, `7px` input/button radius, `24x24` stroke SVG icons, restrained semantic color, and monospace technical fields) based on visual inspection of the `7450451b` Windows Release build and direct code verification.

## 2. Verified Code Findings vs. Screenshot Observations

All items below were verified directly against `desktop/src/` at `7450451b`:

| Area | Verified Code State | Action Required |
|---|---|---|
| **Record Import (`PreviewAcceptPage`) container & layout** | `desktop/src/pages/preview-accept-page.jsx` renders `<div className="settings-layout"><section className="settings-section">` directly without `<Card>`, `<CardHeader>`, or `<CardContent>`, and has a misplaced `.settings-field` wrapper around `#record-import-root` inside `.settings-fields`. | Wrap the export/import sections in `<Card>` / `<CardHeader>` / `<CardContent>`, align the two-column form structure, and remove excessive vertical/inter-column dead space. |
| **Record Import `<textarea>` styling & monospace font** | `desktop/src/style.css` styles `.settings-field input, .settings-field select` but has **no `textarea` rule**, leaving `#record-import-records` with browser default border (`#767676`), sharp corners, missing padding, and default font fallback. | Add `.settings-field textarea` rules matching `input`/`select` (`border: 1px solid var(--border-strong)`, `border-radius: 7px`, `padding: 10px`, `background: var(--surface)`, `font: 12px/1.6 var(--font-mono)`, `resize: vertical`). |
| **Record Import button hierarchy & disabled appearance** | `导出 JSON`, `导出 YAML`, and `接受导入` use `<Button>` (`variant="default"`, `#171717`). Because `.ui-button:disabled` sets `opacity: .5`, disabled primary buttons render as mid-grey (`~#878787`) white-text blocks that look half-enabled. | Use `variant="outline"` for secondary export buttons (`导出 JSON`, `导出 YAML`), keep primary `#171717` only for `接受导入`, and refine disabled button tokens so disabled state is unmistakably distinct. |
| **Record Import copy & root path input** | `PreviewAcceptPage` description reads `"预览待导入记录的媒体证据，再在维护守护下单事务接受。"` (unnatural phrasing). `#record-import-root` uses body font and Linux-style placeholder `"/archive"`. | Polish subtitle (e.g. `"预览待导入记录的媒体证据，并在维护模式下以单事务提交导入。"`) and apply monospace font styling to `#record-import-root` (with platform-neutral/clear placeholder). |
| **Native `<select>` chevron** | `.settings-field select` and `.logs-toolbar select` use native browser dropdown arrows across pages. | Style `select` with `appearance: none` and an aligned `24x24`-style stroke SVG chevron in `var(--foreground-muted)`. |
| **Dashboard "查看所有下载任务" full-width stretch** | In `desktop/src/pages/dashboard-page.jsx`, `<CardContent className="jobs-content">` is `display: flex; flex-direction: column;` (`style.css`), stretching its direct `<Button variant="outline">` child to 100% card width. | Keep the button compact (`align-self: flex-start` or place inside `.button-row`) to match site-wide button proportions. |
| **Dashboard `运行环境` density, alignment & inverted type scale** | In `dashboard-page.jsx`, `.control-panel-content` has `gap: 16px` across four `.dashboard-component-link` items (`12px`, no status dot/alignment column). Meanwhile `.dashboard-detail` has **no CSS rule in `style.css`**, inheriting `16px` body size — larger than the `12px` component rows and `14px` status titles above it. | Define `.dashboard-detail` (`font-size: 12px; line-height: 1.5; color: var(--foreground-muted)`), tighten `.dashboard-component-link` spacing/alignment with monospace component names (`gallery-dl`, `aria2`) and compact status indicators. |
| **Jobs page (`DownloadJobsPage`) empty pagination & refresh placement** | `desktop/src/pages/download-jobs-page.jsx` always renders pagination (`上一页` / `第 1 / 1 页` / `下一页`) even when `totalJobs === 0` (`pageCount <= 1`), and places `刷新` inside the `<section>` card header instead of `<PageHeader action={...} />`. | Hide pagination when `pageCount <= 1` (or `totalJobs === 0`), and unify `刷新` button placement in `<PageHeader>` across pages. |
| **Empty state icon box & card border note (clarification)** | `EmptyJobs` (`desktop/src/pages/shared.jsx`) uses the **same** `.empty-icon` (`border: 1px dashed var(--border-strong)`) on both Dashboard and Jobs pages (screenshot scaling made Dashboard look solid). | No solid-vs-dashed code mismatch exists; keep `.empty-icon` unified and ensure `DownloadJobsPage` avoids redundant outer-vs-inner framing when empty. |
| **Switch (`Toggle`) off-track contrast** | `.ui-toggle-field > input.ui-toggle-input` (`style.css`) uses `background: var(--foreground-muted)` (`#737373`) plus `1px solid var(--border-strong)` in its off state, appearing overly heavy on white surfaces. | Soften unchecked track to `var(--surface-hover)` / `var(--border-strong)` without heavy border contrast, keeping flat shadcn-like restraint. |
| **Semantic color restraint on zero metrics & surface layering** | `DashboardPage` and `BatchesPage` pass `accent="green"` / `accent="red"` unconditionally to `MetricCard` even when `value === 0`. Also, `.main-panel` inherits `var(--surface)` (`#fff`), matching `.ui-card` (`#fff`) with zero surface elevation contrast. | Only apply `accent="green"` / `accent="red"` when `value > 0` (keep `0` neutral `var(--foreground)`), align `--success` hue with `#2e6f59` where appropriate, and set `.main-panel` background to `var(--surface-subtle)` (`#fafafa` light / `#202328` dark) so `#fff` cards stand out cleanly. |
| **Native wide scrollbar (`14px`)** | `.main-panel` (`overflow: auto`) renders default ~14px Windows scrollbar tracks even when content barely overflows. | Apply subtle thin scrollbar styling (`scrollbar-width: thin` and restrained `::-webkit-scrollbar` tokens) matching light/dark themes. |
| **Date input placeholder (`yyyy/mm/日`)** | `BatchesPage` uses native `<input type="date" />`, which Windows WebView2 localizes as `yyyy/mm/日`. | Cross-platform Owner to evaluate keeping native date semantics with locale/UI polish or an accessible custom date input if mixed-locale format is undesirable. |

## 3. Verified Code Findings — Download Config Page (`DownloadConfigPage` & `SettingsSection`)

Verified directly against `desktop/src/pages/download-config-page.jsx`, `desktop/src/components/settings-section.jsx`, and `desktop/src/style.css` at `7450451b`:

| Area | Verified Code State | Action Required |
|---|---|---|
| **Error message rendered in green (`--success`)** | `download-config-page.jsx` renders `outputSettingsMessage`, `downloaderSettingsMessage`, `exchangeMessage`, and `useAria2Message` exclusively with `<p className="settings-message">` (`color: var(--success)`). Failures (e.g. `参数设置加载失败：gallery-dl arguments must be a JSON string array`) do not switch to `.settings-message-error` or `<Alert>`. | Detect error/failure state (or pass structured status severity) and render failures with `.settings-message-error` / `<Alert>` (`--danger` + `--danger-surface`). |
| **Missing `<Card>` borders & asymmetric section grouping** | `SettingsSection` (`settings-section.jsx`) renders `<section className="settings-section settings-panel">` without `.ui-card` (only `border-top` from `.settings-section`). In `DownloadConfigPage`, the top three panels (`归档输出设置`, `下载器参数`, `配置交换`) sit directly under `<>` with no `.settings-layout` wrapper or group heading, while `下载方式` and `aria2` sit inside `<section id="download-settings">` with `<h2>下载设置</h2>` and `.settings-layout`. | Apply `.ui-card` boundaries (`1px` `--border`, `10px` radius) consistently to collapsible panels, and unify top-level section grouping and spacing in `DownloadConfigPage`. |
| **`<textarea>` unstyled in Downloader Arguments & Configuration Exchange** | `#gallery-dl-arguments`, `#aria2-arguments`, and `#configuration-exchange-document` rely on the same missing `.settings-field textarea` CSS rule, resulting in native `#767676` sharp borders, zero padding, and non-monospace text. | Covered by the shared `.settings-field textarea` rule (`border-radius: 7px`, `padding: 10px`, `--font-mono`). |
| **Mixed disabled button styles & spread-out action rows** | In Configuration Exchange, `.batch-form-options .ui-button { margin-left: auto }` (`style.css`) splits the three buttons (`导出本机配置`, `预览导入`, `应用导入`) across left, center, and right edges. In both Configuration Exchange and `aria2`, disabled `default` buttons (`应用导入`, `保存`) appear mid-grey (`opacity: .5` on `#171717`) next to disabled `outline` buttons (`预览导入`, `校验`) with white background. | Use `.button-row` (left-aligned with consistent `8px` gap) instead of `.batch-form-options` for multi-button action bars, avoid mixing confusing disabled visual weights in the same row, and reconsider pushing `.aria2-download-button` far right when primary path actions sit left. |
| **`aria2` header icon order & `下载并安装` icon side** | `Aria2Settings` passes `actions={<Button ...><Icon name="refresh" /></Button>}` to `SettingsSection`, placing a refresh icon right next to the collapse chevron, and places `<Icon name="download" />` on the **right** side of `下载并安装` (contrary to left-side icons elsewhere). | Keep `<Icon name="download" />` on the left of button text to match other buttons, and decide whether the `aria2` refresh action belongs inside the panel body (`aria2-status-row`) instead of beside the collapse chevron. |
| **Output settings vertical spacing & Switch `--success` hue** | In `归档输出设置`, `<div className="settings-fields">` is followed immediately by `<div className="batch-form-options">` with no top margin, crowding `媒体命名` against `导出 tweet.json` / `导出 tweet.yaml` toggles. Unchecked `.ui-toggle-input` uses dark `#737373` and checked uses bright `--success` (`#087f45`) instead of `#2e6f59`. | Add vertical spacing between `.settings-fields` and toggle rows, soften unchecked toggle track, and align toggle checked accent with `#2e6f59`. |
| **Inline technical terms & long prose paragraphs** | Terms like `tweet.json`, `tweet.yaml`, `gallery-dl`, `aria2`, `aria2c`, and `SHA-256` render in body font; `.settings-help` paragraphs in `归档输出设置` and `配置交换` span full width with dense multi-sentence prose. | Wrap inline technical identifiers/filenames in `<code>` (`--font-mono`) and condense or structure long `.settings-help` explanations. |

## 4. Verified Code Findings — Settings Page (`SettingsPage` & `TelegramSettings`)

Verified directly against `desktop/src/pages/settings-page.jsx`, `desktop/src/components/telegram-settings.jsx`, `desktop/src/components/settings-section.jsx`, and `desktop/src/style.css` at `7450451b`:

| Area | Verified Code State | Action Required |
|---|---|---|
| **Nested dual scrollbars** | `.app-shell` sets `height: 100vh; overflow: hidden;` and `.main-panel` sets `overflow: auto;`, yet expanding tall Settings sections in Windows WebView2 produces both an outer viewport scrollbar and an inner `.main-panel` scrollbar. | Audit outer `html`/`body`/`.app-shell` vs `.main-panel` scroll containment so only a single thin scroll container (`main-panel`) scrolls. |
| **Missing `<Card>` borders & inconsistent form widths** | `SettingsSection` lacks `.ui-card` borders/radius across all 8 Settings sections. In `TelegramSettings`, `Endpoint 模式` occupies half a row in `.telegram-settings-grid` while `API 地址` spans `.telegram-field-wide` (`1 / -1`, full width); meanwhile `ProxySettings` `路由检测地址` sits inside `.settings-fields.proxy-diagnose-fields` (`max-width: 600px`), creating inconsistent URL input widths. | Apply `.ui-card` containers consistently to `SettingsSection`, and standardize form grid widths/max-widths for URL inputs and single-column fields. |
| **Mixed disabled button weights & inconsistent Save hierarchy** | `保存日志设置` (`settings-page.jsx`) uses `variant="outline"` while `保存设置` (`telegram-settings.jsx`) and `保存代理设置` (`settings-page.jsx`) use `variant="default"`. In Extension (`注册 / 修复 Native Host`) and Telegram `连接检查` (`检查认证`, `检查目标`, `发送测试消息…`, `验证并替换凭据`), primary `default` buttons sit next to `outline` buttons (`取消注册`, `删除凭据`), making disabled `default` actions appear mid-grey (`opacity: .5` on `#171717`) beside white-background outline buttons. | Standardize all primary Save buttons on `variant="default"`, demote secondary/inspection actions to `variant="outline"`, unify disabled appearance, and style/separate destructive `删除凭据` with `--danger` semantics. |
| **Native `<details>` triangle & header refresh icon** | `ProxySettings` `<details className="proxy-system-details"><summary>当前系统代理配置</summary>` keeps the browser default `▶` disclosure marker (unlike `.extension-guide summary` which hides it). `浏览器 Extension` passes `actions={<Button ...><Icon name="refresh" /></Button>}`, placing a refresh icon beside the collapse chevron. | Replace the native `<summary>` triangle with a stroke SVG chevron (`24x24` style) and align header refresh placement with `aria2`/site conventions. |
| **Core Bootstrap dashed box & Telegram plain status cards** | `ComponentBootstrapStatus` renders `.dependency-missing` (`border: 1px dashed var(--border-strong)`) even for ready catalog state, whereas `Sidecar`/`Extension`/`日志目录` use solid borders. `.telegram-service-status` renders 3 status columns (`凭据服务`, `Bot Token`, `Sender`) as plain black `<strong>` text with no semantic color or status dot. | Use solid subtle status framing for non-missing Bootstrap states, and add semantic status dots/badges (`--success` / `--warning` / `--danger` / muted) to `.telegram-service-status` and Extension status cards. |
| **Monospace tech identifiers, mixed-locale proxy copy & 9-metric diagnostic string** | `https://api.telegram.org` placeholders/inputs, `catalog 2026-09-20-u9`, `logs`, `gallery-dl`, and `aria2` use body font. `ProxySettings` renders English `proxyCoverageText` / backend explanation concatenated directly with Chinese text. `Extension` `.extension-websocket-diagnostics` joins 9 counters with ` · ` into a wrapping sentence. | Apply `--font-mono` / `<code>` to URLs, catalog versions, and technical identifiers; localize the mixed English/Chinese proxy explanation; and render the 9 diagnostic counters in a compact tabular-nums grid instead of a ` · `-joined paragraph. |

## 5. Ownership and Validation Scope

1. **Cross-platform Owner (Linux)**: implements the shared `desktop/src/style.css`, `desktop/src/pages/*.jsx`, and `desktop/src/components/*.jsx` updates above (`CROSS_PLATFORM_CHANGE_REQUIRED`) and runs existing frontend unit/build checks.
2. **Windows Platform Owner (Windows)**: after Git handoff, performs Windows WebView2 GUI verification across Dashboard, Account Batch, Jobs, Download Config, Settings, and Record Import pages (light/dark themes, DPI scaling, keyboard focus, error-state rendering, and zero-state vs populated-state checks).

## 6. Implementation record — 2026-11-22 (Linux/shared)

Every row below was implemented in `desktop/src/` on
`codex/windows-validation-7450451b`, based on Windows source `28075a4`
(`7450451b` + W3/W6 validation write-back `53d5ef0`). Nothing in this section is
Windows GUI evidence; §5.2 acceptance is still `NOT_RUN`.

### 6.1 Record import (`PreviewAcceptPage`)

- Export and import are two `<Card>` / `<CardHeader>` / `<CardContent>` panels; the
  misplaced `.settings-field` wrappers are gone and the export form is a real
  two-column `.settings-fields` grid (Job/Tweet/Batch ID + record type).
- Subtitle: `预览待导入记录的媒体证据，并在维护模式下以单事务提交导入。`
- `#record-import-root` (and the three ID fields) use `.mono-input`; the placeholder is
  platform-neutral (`例如 /archive 或 D:\Archive`).
- `导出 JSON` / `导出 YAML` are `variant="outline"`; only `接受导入` keeps the primary
  fill. Export availability is one named `exportDisabled` expression instead of a
  duplicated inline condition.
- Preview and accept results render inside their own cards with a `.preview-table`
  (header border, mono identity column).
- `#record-import-records` is styled by the new shared textarea rule; the action bar is
  `.button-row`, not `.batch-form-options`.

### 6.2 Global form controls and surfaces

- `.settings-field textarea`: `1px` `--border-strong`, `7px` radius, `10px` padding,
  `--surface` background, `font: 12px/1.6 var(--font-mono)`, `resize: vertical`.
- `.settings-field select` and `.logs-toolbar select` use `appearance: none` plus one
  stroke-SVG chevron (light/dark variants), aligned at `right 10px center`.
- Toggle track: unchecked `var(--surface-hover)` (no heavy `--foreground-muted` fill),
  checked `var(--accent)` (`#2e6f59` light / `#3f9077` dark) instead of the status
  `--success`; the knob carries a small shadow so it reads on the light track.
- `.main-panel` uses `var(--surface-subtle)` and a thin scrollbar (`scrollbar-width:
  thin` + `::-webkit-scrollbar` 10px, themed thumb). `html, body, #root` are `100%`
  high and `body` is `overflow: hidden`, so `.main-panel` is the only scroll container.
- `.ui-button:disabled` no longer renders the primary fill at 50% opacity; disabled
  buttons drop the fill and keep muted text. `.ui-button-destructive` is defined
  (danger text/border/surface hover) — it existed in the variant map with no CSS.
- `.settings-fields + .batch-form-options` / `.settings-fields + .button-row` get a
  `16px` gap, separating toggle rows from the fields above them.
- `code` inside `.settings-help` / `.proxy-help` / `.aria2-help` is monospace with a
  subtle surface chip.

### 6.3 Dashboard, Jobs

- `查看所有下载任务` sits in a `.button-row jobs-footer` with `align-self: flex-start`,
  so the column flex no longer stretches it to the card width.
- `.dashboard-detail` is defined (`12px`, muted) — it previously inherited `16px` and
  inverted the type scale against the `12px` component rows.
- Component links are one `.dashboard-component-list` grid of
  `dot / mono name / state`, with `data-tone` (`ok`/`warn`/`off`) driving the dot color.
- `下载方式` no longer prints the long parenthetical; the state column carries
  `aria2` / `gallery-dl`.
- Metric accents: `已完成`/`失败` pass `accent` only when the value is `> 0`
  (`accent="default"` otherwise), on both Dashboard and Account Batch. A green or red
  zero is no longer rendered.
- Jobs page: `刷新` moved into `<PageHeader action>`, pagination renders only when
  `pageCount > 1`, and the empty state keeps one card frame around `EmptyJobs`
  (verified: Dashboard and Jobs already shared `.empty-icon`; no solid/dashed mismatch
  existed).


### 6.4 Download config

- `outputSettingsMessage`, `downloaderSettingsMessage`, `exchangeMessage` and
  `useAria2Message` render through `statusMessageClass(...)`, so failures use
  `.settings-message-error` instead of the success color.
- The top three panels sit in a `.settings-layout`; every `SettingsSection` is now a
  card (`.settings-panel`: `1px` border, `10px` radius, surface background), matching the
  `下载设置` group.
- Configuration-exchange actions use one left-aligned `.button-row`
  (`导出本机配置` / `预览导入` outline, `应用导入` primary, `取消等待` outline when busy).
- aria2: the refresh action left the collapse chevron and now sits at the end of
  `.aria2-status-row`; `下载并安装` puts `<Icon name="download">` on the left; the
  desktop `margin-left: auto` on `.aria2-download-button` is removed so the primary
  download action is not pushed to the far edge.
- Technical terms in the argument and exchange help text are wrapped in `<code>`.

### 6.5 Settings

- `保存日志设置` is a primary (`default`) button, matching `保存设置` / `保存代理设置`.
  Telegram inspection actions (`检查认证`, `检查目标`, `发送测试消息…`) are `outline`;
  `验证并替换凭据` stays primary; `删除凭据` is `variant="destructive"`.
- Extension panel: the header refresh beside the collapse chevron is removed; the
  refresh lives in `.extension-actions` (`刷新 Extension 状态`) and the page header keeps
  its own `重新检测`.
- `网络代理` copy is Chinese only: `proxyCoverageText` no longer concatenates the English
  backend note (the previous text produced `enabled.暂不支持…`). The backend still
  reports the same facts; the UI states the policy itself.
- The nine WebSocket counters render as a `<dl>` grid (3 columns, tabular numerals)
  instead of a ` · `-joined sentence.
- `Core Bootstrap` uses `.dependency-status` (solid) and only adds
  `.dependency-missing` (dashed) when the catalog is not ready; the catalog version is
  `<code>`.
- Telegram service status uses a dot + label + value grid with `data-tone`
  (`ok`/`warn`/`off`) and semantic value colors.
- Both `<details>` disclosures (`当前系统代理配置`, `Extension` guide) hide the native
  triangle and draw one stroke chevron that rotates on open (`list-style: none`,
  `::-webkit-details-marker`, `::after`).
- `代理地址`, `路由检测地址` and `API 地址` inputs use `.mono-input`; `logs`,
  `gallery-dl`, `aria2` in help text are `<code>`.

### 6.6 Deliberate non-changes

- **Native date inputs** (`开始日期` / `结束日期`) keep native semantics. A custom date
  picker would re-implement locale, keyboard and validation behavior that WebView2
  already provides; the `yyyy/mm/日` placeholder is a platform locale decision, not a
  defect. Recorded so a later UI pass does not reverse it silently.
- **`.control-panel-content { gap: 16px }`** is unchanged; the component rows are
  tightened inside their own list (`gap: 2px`) so the existing spacing contract holds.
- **`.empty-icon`** stays dashed on both pages; no solid/dashed mismatch existed.

### 6.7 Validation (Linux)

| Check | Result |
|---|---|
| Desktop Node `npm test` | **PASS** — 276 tests, 273 pass, 3 Windows-only skip, 0 fail (14 new source/render assertions for the items above) |
| `npm run check` (Vite build) | **PASS** (existing non-blocking dynamic-import chunk warning only) |
| `node scripts/check-architecture.mjs` | **PASS** — 0 findings after the reviewed baseline reseal for the changed `desktop/src` files |
| `node scripts/docs-audit.mjs` | **PASS** |
| `git diff --check` | **PASS** |

Not run: any Windows build or WebView2 render. DPI, dark-theme contrast, keyboard focus
order and visual acceptance remain Windows-owned (§5.2).

### 6.8 Windows acceptance still required

1. Record Import: both cards, the two-column export grid, the mono root field and the
   outline exports at 100/125/150/200% DPI.
2. Download Config: error messages in `--danger`, one left-aligned action row, aria2
   refresh inside the body, panel card borders aligned with `下载设置`.
3. Settings: a single scrollbar (no outer viewport scrollbar), the
   primary/secondary/danger button hierarchy, stroke chevrons on both disclosures,
   Bootstrap solid vs dashed, Telegram status dots, the 3×3 diagnostics grid, and no
   mixed-language proxy copy.
4. Dashboard/Jobs: compact `查看所有下载任务`, `.dashboard-detail` at 12px, neutral zero
   metrics, aligned component rows, no pagination for a single page, refresh in the page
   header.
5. Light and dark themes, keyboard focus visibility, and populated vs empty states.

## 7. Follow-up Visual Review Findings — 2026-11-24 (200% DPI Windows Screenshots)

From manual 200% DPI screenshots of `4e427afc` (`78b508cd...`), verified against `desktop/src/` (`CROSS_PLATFORM_CHANGE_REQUIRED`, Cross-platform Owner):

- **Confirmed code changes needed**:
  - **Error message container (`DownloadConfigPage` / `style.css`)**: `.settings-message-error` renders bare red text tightly above action buttons instead of a padded `--danger-surface` + `7px` radius alert strip (like `.widget-error`).
  - **Disabled `<textarea>` styling (`style.css`)**: `.settings-field textarea` lacks `:disabled` / `:read-only` styling, leaving `#gallery-dl-arguments` with `#fff` background and `resize: vertical` handle; set `background: var(--surface-muted); resize: none;` when disabled.
  - **Account Batch empty state (`BatchesPage`)**: `还没有账号归档批次。` renders as a plain left-aligned `<CardContent>` line instead of the centered icon + title + subtitle empty-state pattern used by `EmptyJobs`.
  - **Storage single-card page (`StoragePage`)**: uses collapsible `<SettingsSection>` with a chevron even though `存储位置` is the only card on the page; switch to a non-collapsible `<Card>`.
  - **Custom aria2 path display (`DownloadConfigPage`)**: before picking a custom path, `自定义 aria2 路径` renders only buttons with no read-only mono path display/placeholder bar.
  - **Output settings vertical spacing & group symmetry (`DownloadConfigPage` / `style.css`)**: check `.settings-fields + .batch-form-options` specificity so `媒体命名` has `12–16px` breathing room above `导出 tweet.json`/`导出 tweet.yaml`; add matching top group heading or remove asymmetric `下载设置` heading; tighten `.settings-help` prose and avoid orphan 2-char line wrap in `.dashboard-detail` (`查看。`).
  - **Focus ring hue (`style.css`)**: `--focus-ring` is `#0067c0` (Windows blue); Cross-platform Owner to decide whether to align with `--accent` (`#2e6f59`) or keep platform blue.
  - **Disabled button border consistency (`style.css`)**: `.ui-button-outline:disabled` retains `1px` `--border`, while `.ui-button-default:disabled` has `border-color: transparent` (e.g. `检查认证`/`检查目标`/`发送测试消息…` with border next to `验证并替换凭据` without border in Telegram `连接检查`). Unify `.ui-button:disabled` border/surface tokens.
  - **Input width standards (`style.css`, `telegram-settings.jsx`, `settings-page.jsx`)**: `.telegram-credential-field` (`新 Bot Token`, `max-width: 520px`) and `.proxy-diagnose-fields` (`路由检测地址`, `max-width: 600px`) match neither the 2-column `50%` grid nor `100%` full width; align to `50%` or `100%`.
  - **Textarea native resize grip (`style.css`)**: `.settings-field textarea` currently sets `resize: vertical`, which still renders WebView2's diagonal resize grip in fixed-width cards; consider `resize: none` or custom-sized rows where appropriate.
- **Clarifications (no code defect)**:
  - Toggle checked track in `style.css` already uses `var(--accent)` (`#2e6f59` light / `#3f9077` dark) across all pages (including `运行日志` and `下载配置`).
  - Dashboard and Jobs empty-state `.empty-icon` already share `1px dashed var(--border-strong)`.
  - `ComponentBootstrapStatus` (`connection-status.jsx`) already wraps `{bootstrap.catalog_version}` (`2026-09-20-u9`) in `<code>`.
  - Native `<input type="date">` (`yyyy/mm/日`) remains intentionally unchanged per §6.6.

## 8. Implementation record — Round-2, 2026-11-24 (Linux/shared)

Every §7 "code changes needed" item was implemented in `desktop/src/` on
`codex/windows-validation-7450451b` over source `4e427afc`. None of this section is
Windows GUI evidence; §7 / §6.8 acceptance stays `NOT_RUN` for the Windows Owner.
No Tauri IPC command, storage schema, or sidecar/download protocol changed.

### 8.1 R1 — error messages render as an alert strip

- `.settings-message-error` (previously bare text) is now a padded alert block:
  `border: 1px solid #f0b8b5`, `border-radius: 7px`, `padding: 9px 11px`,
  `background: var(--danger-surface)`, `color: var(--danger)`, matching `.widget-error`.
- A scoped `margin-top: 12px` rule separates the strip from the action row above it in
  `.settings-panel-content` / `.ui-card-content` / `.download-job-entry`.
- `statusMessageClass(...)` already routed failures here since §6.4; only presentation
  changed.

### 8.2 R2 — disabled / read-only textarea

- `.settings-field textarea` now sets `resize: none`, so WebView2 no longer paints the
  diagonal grip that clashed with the `7px` radius inside fixed-width cards (height comes
  from `rows`).
- `.settings-field textarea:disabled, .settings-field textarea:read-only` drop to
  `color: var(--foreground-muted)`, `background: var(--surface-muted)` and
  `cursor: default` — `#gallery-dl-arguments` is no longer a white editable-looking box.

### 8.3 R3 — one disabled button primitive

- The two overlapping disabled rules collapsed into one selector covering
  `default`, `secondary`, `outline`, `ghost`, `destructive` and icon-only variants:
  `color: var(--foreground-muted)`, `border-color: var(--border)`,
  `background: var(--surface-muted)`, `box-shadow: none`.
- `.ui-button-destructive:disabled` is folded into the same primitive so a disabled
  destructive action no longer keeps danger-colored text.
- The `opacity: .5` primary fill is not reintroduced anywhere (asserted by a regression).

### 8.4 R4 — input width standard

- `.telegram-credential-field` (`新 Bot Token`) goes from `max-width: 520px` to
  `max-width: none`, matching the other Telegram fields' two-column grid.
- `.proxy-diagnose-fields` keeps one column but drops `max-width: 600px` to
  `max-width: none`, so `路由检测地址` uses the full card width instead of a third
  standard.

### 8.5 R5 — empty and single-panel states

- `shared.jsx` gains `EmptyState({ icon, title, detail })`; `EmptyJobs` is now a thin
  wrapper over it, so Dashboard and Jobs keep identical markup.
- `BatchesPage` renders the centered `EmptyState` (`还没有账号归档批次` +
  `创建批次并开始发现后，候选 Tweet 会显示在这里。`) inside its card instead of a
  plain left-aligned line of text.
- `SettingsSection` treats a missing `onToggle` as non-collapsible: it renders
  `settings-panel-static` with a `settings-panel-static-header` `role="group"` and no
  chevron. `StoragePage` passes no `onToggle`, so `存储位置` is a static card on a
  single-panel page.

### 8.6 R6 — layout symmetry and prose wrapping

- `DownloadConfigPage` wraps both halves in symmetric group sections:
  `<section id="output-settings-group" aria-labelledby>` +
  `<h2 class="download-section-heading">输出与配置</h2>` mirrors the existing `下载设置`
  heading; `.download-section-heading` is shared by both.
- `.settings-fields + .batch-form-options, .settings-fields + .button-row` spacing goes
  from `16px` to `20px`, giving `媒体命名` breathing room above the export toggles.
- `.dashboard-detail` adds `text-wrap: pretty`, avoiding the orphan two-character wrap
  (`查看。`) seen at 200% DPI.
- `.aria2-path-controls` shows a read-only `PathDisplay` (`当前生效的 aria2 可执行文件`,
  falling back to `尚未检测到 aria2 可执行文件`) when no custom path is staged, instead of
  an action row with no path context.

### 8.7 R7 — focus ring decision

- Decision: `--focus-ring` stays `#0067c0` (platform blue). It is the OS focus color the
  WebView2 host and native controls already use; aligning it to brand `--accent`
  (`#2e6f59`) would make keyboard focus read as a semantic "success" state. `--accent`
  remains the brand green for checked toggles and primary emphasis. Recorded here so a
  later pass does not flip it silently.

### 8.8 WDIO specs approved for this batch

Both are opt-in and skip unless the platform, opt-in flag and isolated-root marker all
match; neither runs in the default `npm test` path.

| Spec | Opt-in flag | Fixture marker | Contract |
|---|---|---|---|
| `desktop/e2e/specs/transfer-switch-held-busy.e2e.mjs` | `WDIO_TRANSFER_HELD_BUSY_GUI=1` | `w7-transfer-held-busy-fixture.marker` = `isolated-transfer-held-busy` | Start the save, then dispatch one non-repeat plus four repeat `Enter` keydowns: the switch must stay disabled, exactly one write must land, and the value must still be `use_aria2: true` after a 1s settle with focus returned to the switch. |
| `desktop/e2e/specs/transfer-switch-moved-focus.e2e.mjs` | `WDIO_TRANSFER_MOVED_FOCUS_GUI=1` | `w7-transfer-moved-focus-fixture.marker` = `isolated-transfer-moved-focus` | Move focus to the panel toggle in the same task as the click: the settle must not steal focus back, `Enter` must collapse the panel rather than re-apply the switch, and `config.yaml` must still read `use_aria2: true`. |

Both require `XARCHIVE_PORTABLE_ROOT` pointing at an isolated disposable root; they must
never target the owner's canonical working tree.

### 8.9 Validation (Linux)

| Check | Result |
|---|---|
| Desktop Node `npm test` | **PASS** — 284 tests, 281 pass, 3 Windows-only skip, 0 fail (Round-2 source/render assertions for R1–R6) |
| `cargo test --workspace --offline` | **PASS** |
| `cargo fmt --all -- --check` | **PASS** |
| `node scripts/check-architecture.mjs` | **PASS** — 0 findings after the reviewed baseline reseal for the changed `desktop/src` files (no entries added or removed) |
| `node scripts/docs-audit.mjs` | **PASS** |
| `git diff --check` | **PASS** |

Not run: any Windows build, WebView2 render, or execution of the two new WDIO specs.
R1–R7 visual acceptance, 200% DPI eye-check, non-zero Dashboard/Jobs metrics and the
missing-catalog `.dependency-missing` state remain Windows-owned
(`docs/validation/windows-queue.md`).
