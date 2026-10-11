# RiverLens

**v0.4.0：MDA 群體分析＋AI 證據建議**。支援 GG 一般桌 USD NLH 6-max、獨立 DuckDB 資料集、情境統計與匿名回放，內建 Coach／MCP 共用證據工具。資料集分享預設關閉。[版本詳情](docs/release-v0.4.0.md) · [統計定義與平台限制](docs/mda-population.md)。

**您的牌譜，由您掌握。以本機資料為本的賽後 Poker 複盤工作台。**

[English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Türkçe](README.tr.md)

RiverLens 是 Natural8／GGPoker 個人現金桌牌譜與獨立 MDA 群體資料分析桌面工具。介面使用 React／TypeScript，Rust core 負責解析及統計；個人資料存 SQLite，MDA 資料集存 DuckDB。

> **[下載 v0.4.0](https://github.com/G-Loop-co/riverlens/releases/tag/v0.4.0)** — MDA＋AI、本機 MCP 及離線學習。提供 Windows、Apple Silicon、Intel Mac 套件。macOS 為 ad-hoc 簽章，未經 Apple notarization。[發布說明](docs/release-v0.4.0.md)。

![RiverLens 繁體中文總覽：合成資料](docs/screenshots/overview-zh-TW.jpg)

README 共 **13 種語言**；個人複盤介面支援 **English／繁中／簡中**，首版 **MDA 工作區標籤使用繁體中文**。

**v0.4.0** 新增群體研究，保留 AI 個人複盤及 Spot Explorer → Leak Finder → 翻前策略對照 → Trainer。策略包需自行提供；頻率對照不是 decision EV。[學習工作台](docs/study-workflow.md) · [主題](docs/theme-selection.md)。

## 功能

| 功能 | 用途 |
| --- | --- |
| AI 教練與 agent 連接 | 透過 23 個本機 MCP 工具查閱統計與決策情境，建立附手牌依據的複盤、練習及學習計劃草稿，由您確認。 |
| MDA 群體分析 | 匯入獨立 GG 一般桌資料集；按情境分析全桌合法機會、Wilson 區間及研究假設，點開匿名手牌驗證。 |
| 匯入中心 | 匯入 TXT、ZIP 或資料夾；查看進度、暫停／續匯、去重及異常隔離。 |
| 盈虧總覽 | 查看淨盈虧、bb/100、Session 與位置分類，點入原始手牌。 |
| 13 項核心統計 | VPIP、PFR、RFI、3-bet、盲位防守及翻後／攤牌頻率；分子、機會分母可分別查閱。 |
| 13 × 13 起手牌矩陣 | 查看實際樣本、net bb、bb/100、行動頻率，開啟對應手牌。 |
| 手牌回放 | 逐步行動、跳街、自動播放及已知底牌。 |
| 複盤工作台 | 保存筆記、標籤、已複盤狀態及常用篩選。 |
| 本機資料管理 | 個人 SQLite、獨立 MDA DuckDB；個人 CSV／牌譜匯出，各資料庫獨立備份還原。 |
| 外觀主題 | 森林綠、午夜藍、暖白；選擇儲存於本機。 |
| 個人複盤三語介面 | 繁中、簡中及 English，離線可用；MDA 工作區目前使用繁體中文標籤。 |
| 適用 All-in equity | 支援單挑、單底池、已知底牌、單 runout 的適用情況；不是 decision EV 或 GTO 評分。 |

![起手牌矩陣：合成資料](docs/screenshots/starting-hands-en.jpg)

截圖採 **1920px 桌面寬度、完整頁面、暖白主題**；另提供 [1920 × 1080 完整視窗](docs/screenshots/overview-desktop-en.jpg)。

截圖全部使用 **240 手合成資料**，不包含私人牌譜。重複樣本僅示範介面，不代表真實玩家頻率或策略建議。矩陣呈現已觀察手牌，並非建議範圍。

## MDA＋AI 快速上手

1. 開啟「**MDA 群體分析**」，建立資料集並指定來源、牌譜內文時區；匯入 GG 一般桌 USD NLH 6-max 的 TXT、ZIP 或資料夾。匯入頁可暫停／續匯及查看隔離紀錄。
2. 篩選級別、位置對位、pot type、有效籌碼、牌面及面對下注尺寸。翻後研究預設 HU；每個百分比配合合法機會分母及匿名手牌閱讀。
3. 內建 Coach：啟用此資料集的雲端分享及既有 provider 分享同意，選擇 provider／model。外部 agent：啟用此資料集的 MCP 權限及本機連接。兩種資料集權限均預設關閉。
4. 要求適用情境、分子／分母、調整方向及尺寸、理由、例外與匿名案例；在資料集的 AI 頁審閱草稿，再回放證據手牌。

Apple M3 Pro 的 **1,000 萬手合成資料驗收通過**：彙總 p95 **16.4 ms**、情境查詢 **1.20 秒**、程序峰值 RSS **1.39 GiB**。範圍為兩組指定核心查詢、暖 OS cache，不包含桌面 UI／IPC／provider 延遲。[來源、硬體、正確性與完整效能紀錄](docs/validation/mda-population-ai.md)。

沒有基準時標示研究假設；攤牌樣本顯示覆蓋率與選擇偏差。純 bluff 門檻只適用於列明假設的 HU river 情境，不是 Solver 或 decision EV。HHDealer PT／HM／H2N4、真實 provider 及外部模型客戶端仍待真實樣本或實測。

![MDA 群體統計：10,000 手合成資料、繁體中文工作區](docs/validation/mda-stats.jpg)

## AI 功能與快速上手

查閱統計、分析指定決策、找出值得檢討的局面，並建立附手牌依據的練習或學習計劃草稿。草稿需由您確認；AI 與原有離線學習資料庫目前分開。

**使用外部 agent：RiverLens 不需要 provider API key**

1. 匯入已完成的牌譜，保持 RiverLens 開啟。
2. 開啟「**AI 教練 → AI 連接 → 啟用本機連接**」，將顯示的設定複製到支援 **stdio MCP** 的客戶端，例如 Codex。
3. 輸入：「找出最值得檢討的 leak candidates，列出樣本量及手牌依據，再建立練習草稿。」到「**學習資料**」檢視並確認草稿。

外部 agent 可能需要自己的登入／訂閱。重啟 RiverLens 後需更新連接設定；「**撤銷連接**」可停止存取。

**使用 App 內 AI 教練：需要 API key**

在「**AI 連接**」儲存 OpenAI、Anthropic、Gemini、DeepSeek 或 OpenCode Go key，再於「**AI 教練**」選擇相容 model、分享範圍並提問；亦可從手牌回放按「**問 AI**」。

選定內容可能傳送至模型供應商，包括經外部 agent 傳送。Key 存於系統憑證庫。雲端供應商實際呼叫尚未驗證；無 key 仍可使用離線學習。[完整設定與限制](docs/ai-agent-coach.md)。

### 支援的 AI 連接

App 已實作以下五個供應商。請選擇帳戶可用、支援工具呼叫的模型；已實作 API 不代表所有模型均已實測。

| 供應商 | 模型／API | 設定 |
| --- | --- | --- |
| OpenAI | 支援工具的 GPT 模型；Chat Completions | OpenAI API key |
| Anthropic | 支援工具的 Claude 模型；Messages | Anthropic API key |
| Google Gemini | 支援工具的 Gemini 模型；streamGenerateContent | Gemini API key |
| DeepSeek | `deepseek-flash`、`deepseek-v4-pro`；舊別名以供應商實際接受為準 | DeepSeek API key |
| OpenCode Go | GLM、Kimi、LongCat、DeepSeek、MiMo、Hy、MiniMax、Qwen、Grok、GPT、Muse；依模型系列路由 | Go 訂閱及 API key |

**外部 agent：**支援本機 **stdio MCP** 的客戶端（例如 Codex）可使用畫面提供的連接設定讀取 RiverLens 工具。這是協定連接，並非內建供應商，亦不代表每款客戶端均已實測。模型及登入由客戶端提供；此路徑不需要在 RiverLens 儲存供應商 key。

<details>
<summary>OpenCode Go 模型 ID 及自動 API 路由</summary>

| API | 模型 ID |
| --- | --- |
| Chat Completions | `glm-5.3-flash`, `glm-5.3`, `glm-5.2`, `glm-5.1`, `kimi-k3`, `kimi-k2.7-code`, `kimi-k2.6`, `longcat-2.0`, `deepseek-v4.1-flash`, `deepseek-v4-pro`, `deepseek-v4-flash`, `deepseek-v4-flash-vision-exp`, `mimo-v2.5`, `mimo-v2.5-pro`, `hy4-preview`, `hy3` |
| Messages | `minimax-m3`, `minimax-m2.7`, `minimax-m2.5`, `qwen3.8-max`, `qwen3.8-flash`, `qwen3.7-max`, `qwen3.7-plus`, `qwen3.6-plus` |
| Responses | `grok-4.6`, `gpt-5.6-luna`, `muse-spark-1.3-contributor`, `muse-spark-1.2-contributor` |

</details>

模型目錄會變更；目前可用 ID 及權限以 [DeepSeek](https://api-docs.deepseek.com/quick_start/pricing/) 及 [OpenCode Go](https://opencode.ai/docs/go/#endpoints) 為準。輸入純模型 ID，不加 `opencode-go/`。Go 主要面向 coding agent；RiverLens Poker coaching 使用資格及實際相容性尚未驗證。具視覺能力的模型亦只會收到文字。

## 開始使用

需要 Node.js 22+、Rust stable 及 [Tauri 平台前置工具](https://v2.tauri.app/start/prerequisites/)：macOS 使用 Xcode Command Line Tools；Windows 使用 MSVC C++ Build Tools 與 WebView2。

```sh
git clone https://github.com/G-Loop-co/riverlens.git
cd riverlens
npm ci
npm run desktop
```

1. 從 PokerCraft 匯出自己已完成的 TXT／ZIP 牌譜。
2. 在「資料與設定」確認品牌、Hero 名稱及**牌譜內文時區**。
3. 在「匯入中心」匯入檔案。
4. 查看總覽、矩陣及逐手回放。
5. 保存筆記，定期備份資料庫。

## 本機開發

```sh
# Terminal 1：Rust core，只綁定 127.0.0.1
npm run serve:core
# Terminal 2：瀏覽器預覽 http://127.0.0.1:1420
npm run dev
```

桌面版使用 Tauri IPC 及原生檔案對話框；瀏覽器預覽使用相同 Rust core，以開發用路徑輸入代替檔案對話框。

```sh
npm test
npm run core:test
npm run build
node scripts/cargo.mjs clippy -p poker-core --all-targets -- -D warnings
npm run desktop:build -- --bundles app
# Windows 原生 runner：
npm run desktop:build -- --target x86_64-pc-windows-msvc --bundles nsis
```

資料存於 Tauri app-data 目錄（`app.riverlens.desktop`），設定頁會顯示實際位置。瀏覽器開發使用 `.local/riverlens.db`。運作中的 SQLite 請使用內建備份功能，確保 WAL 資料一併處理。

## 範圍與發布狀態

本工具供個人離線賽後複盤與 MDA 群體研究。不連接遊戲客戶端，沒有即時 HUD、RTA、雲端同步或 GTO 最佳行動評分。與 Natural8／GGPoker 無隸屬關係。GG SEP 第 7／19 條限制收集他人牌譜與收集／分享 MDA，可能涉及停權、封禁或資金沒收；詳見 [MDA 功能限制及驗證](docs/mda-population.md)。

v0.4.0 包含 MDA、AI／MCP 整合及離線學習流程；不包含私人牌譜、本機資料庫或策略包。驗收限制見[發布說明](docs/release-v0.4.0.md)。

- [使用說明](docs/user-guide.md)
- [架構與統計定義](docs/architecture.md)
- [研究與既有元件](docs/research-matrix.md)
- [驗證與限制](docs/validation.md)
- [版本紀錄](CHANGELOG.md)
- [第三方元件](THIRD_PARTY.md)
- [Releases](https://github.com/G-Loop-co/riverlens/releases)

## 授權

Repo 公開，但尚未選定專案整體再利用授權；不可假設 RiverLens 本身採 MIT 或 Apache。第三方元件保留各自授權，見 [THIRD_PARTY.md](THIRD_PARTY.md)。
