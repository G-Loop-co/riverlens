# 架構與統計契約

## 資料流

```
TXT / ZIP / folder (UTF-8 streaming)
  → GG parser + explicit source profile / timezone
  → incremental actions + exact ledger + card validation
  → valid / quarantined / unsupported source outcome
  → SQLite: indexed hands, compressed raw payload, per-hand opportunities
  → incremental rollups / paginated queries / HU equity worker
  → typed Tauri request → React reports / matrix / replay / annotations
```

`crates/poker-core` 不依賴 Tauri；相同實作可由桌面、測試、benchmark 及本機開發 server 呼叫。`src-tauri` 只處理原生視窗、檔案對話框、app data 路徑及非同步 IPC。前端不接受／執行 SQL。

## 帳本與完整度

- `i64` 金額，1 USD = 1,000,000 units。decimal parser 不經 f64；IPC 字串，replayer 使用 BigInt。下注比例、equity 機率及 bb 僅為顯示／分析比例。
- 每次 raise-to 轉換成增量；missed/dead blind 不抵扣 live contribution。straddle 是當前總額；退注不算獲利。Cashout／Risk 與底池收支分離。
- SUMMARY 不重複入帳派彩；費用為全桌資料。不能由全桌 rake 推導 Hero rake。未知對手牌不推測。
- 驗證目前投注級別、行動投入、籌碼、退注後底池、派彩與全桌費用、牌張合法性。異常保留 immutable raw 並隔離核心統計。
- Side pot 依不同投入層及尚未 fold 玩家推導；同 eligible set 合併。多 runout 各自重建共用牌面。
- 原文有 SHOWDOWN 字樣不足以判定 Hero showdown；Hero 需仍在牌局、至少兩名未 fold，且有顯牌或 saw-flop 證據。

## SQLite 與背景工作

Schema 2，WAL／foreign keys／短交易／單一寫入 mutex。單次匯入正常批次為 250 手牌；benchmark 為 1,000。存 `(profile, hand_id)`、原文 SHA-256、parser／stats version、所有正規化動作與原文壓縮 payload。大原文放 `hand_payload`，統計不需要讀取／解壓。

`hands` 保存分子與分母；`rollups` 以 exact integer 累加 date、position、stakes、game、hand class、profile 維度。無進階條件的常用報表讀 rollups；複雜條件由索引及 prepared predicate 查原始 per-hand 欄位。查詢多個報表段落時使用同一 SQLite read transaction，防止匯入中讀到不同快照。

Hands 使用 `(sort_value, id)` 游標，page 最多 200，預設 60。複合條件 Report 分組上限 2,000 個群組，有截斷旗標與 UI 提示；不完整分組 CSV 會拒絕匯出。Session 定義為同 profile 的**來源 TXT／ZIP entry**，不是根據玩家休息時長自動推斷。基本日期／位置／stakes／game／hand class／session rollup 沒有截斷。

新增 `slice` rollup，以 JSON tuple 保存位置、stakes、game、來源日期；所有金額與分子／分母均增量更新。當日期篩選時區與來源時區一致，常用組合直接讀此彙總；跨來源時區回退原始 UTC 時間條件，避免日界線錯誤。舊 schema 2 資料庫首次開啟會補建此彙總。`hand_stat_cover` 讓大範圍統計機會查詢讀窄索引頁，避免反覆讀牌譜／長來源路徑；查詢仍以參數化條件為準。

Jobs 保存狀態與檔案完成 checkpoint。取消／意外結束後重掃未完成檔案，以 unique key 保證已提交批次不重複。掃描數是嘗試次數，包含重試。來源檔案移走／改變需由使用者處理，無自動下載。

1 個 equity worker，與匯入分開。枚舉每 4,096 runout 檢查暫停；完成逐手入庫。輸入 hash + algorithm version 快取 equity 與精確 adjusted net；不利用未來實際 board。

備份先在資料庫自身目錄用 SQLite backup API 產生快照，轉為 DELETE journal、關閉 SQLite，再把完整單檔寫到選定位置並 sync；避免外部儲存位置需要額外 journal／WAL／SHM 的存取。還原先把選定備份複製到自身暫存位置，檢查 integrity/schema，並自動備份目前資料。SQLite backup 連續鎖定 15 秒會返回可見錯誤，不無限等待。暫存快照用 RAII 清理。重算使用 staging DB，完成後替換；notes、tags、source profiles、saved filters、jobs、無法解析原文保留。

## 統計定義（hero-opportunities/1.0.0）

所有百分比 = 分子 / 機會 × 100；每手最多計一次。零機會回傳 null。分母設計明確，不聲稱與所有商業 tracker 預設定義完全相同。

| 指標 | 分子 | 機會分母 |
|---|---|---|
| VPIP | 主動 Call／Raise | 可作自願翻前決策；排除 walk、強制盲注／ante |
| PFR | 曾翻前 Raise | 曾有合法加注機會；須仍有可回應對手 |
| RFI | 無 limp／raise 前首先 Raise | 尚未有人主動入池且可合法 Raise |
| 3-bet | 首次翻前 re-raise | 面對一個 Raise 且可合法 re-raise；包括 squeeze |
| Fold to 3-bet | Hero open 後面對 re-raise 而 Fold | Hero 曾 Raise，面對第二次 Raise 的決策 |
| Blind fold／call／raise | 盲位選擇對應行動 | 面對 first-in open，尚無 cold caller，Hero 未先加注 |
| Flop C-bet | 最後 preflop aggressor 在 flop Bet | 輪到該 Hero 且沒有人先下注；donk 後不算 |
| Fold vs C-bet | 面對該 C-bet 而 Fold | Hero 面對對手 C-bet 且尚無 intervening raise |
| WTSD | Hero 到 showdown | Hero saw flop |
| W$SD | Hero 在 showdown 收取至少部分底池（含 tie） | Hero showdown |
| WWSF | Hero 收取至少部分底池 | Hero saw flop |

HU effective stack = 兩名 flop 參與者較小的**起始**籌碼 / BB；多人及未到 flop 無此值，不填零。Flop texture 取 flop 三張牌：rainbow／two-tone／monotone、paired、最高 rank。下注大小條件為該次增量投入 / 行動前底池，raise 不解讀為 raise-to / pot。

## All-in equity

V1 的必要条件：valid ledger、Hero 參與 HU、單底池、單 runout、雙方兩張牌已知、無 Cashout、無 straddle／short-raise 特殊下注、無特殊結算。最後 all-in call／退注之後無其他決策，鎖定當刻 board。所有排除有原因。

`equity = (2 × wins + ties) / (2 × all legal runouts)`。

`adjusted net = equity × (total pot − recorded table fees) − Hero net contribution`，以 i128 rational 中間值計算後四捨五入至 1 micro-unit。實際已扣費派彩不再扣費；不估算每個反事實發牌的不同費率。此約定與可能採不同 rake／promotion 模型的 tracker 未必完全一致。

報表先保留全部有效實際結果，再對 completed eligible hands 套用 adjusted delta。pending、excluded、not_applicable 個別標記，沒有替缺少資料填 equity 0。

## 介面國際化

採用 i18next／react-i18next，三份 JSON 詞典隨前端打包，不使用遠端翻譯服務。`src/i18n.ts` 初始化 `zh-TW`、`zh-CN`、`en`，使用 `riverlens-language` 保存本機偏好；不支援的值回退至繁體中文。每次切換更新文件 `lang`／標題，組件透過 `useTranslation` 重新呈現。數字與日期依語言顯示，手牌清單仍固定 HKT。

繁體與英文詞典維護於 `src/locales/`；`npm run locales:generate` 使用 OpenCC 與 UI 詞彙表產生提交至 Git 的簡體詞典。OpenCC 只用於開發，不進產品 runtime，亦不轉換使用者資料。篩選、行動與狀態仍以引擎的 canonical 值跨 IPC；只有標籤翻譯。已知引擎診斷以明確範本翻譯並保留金額證據，未知訊息原樣顯示。

測試檢查三語鍵與插值一致、英文單複數、書面語字詞、統計定義覆蓋、篩選值不變、語言偏好隔離與原文插值 escaping。實作參考官方 [react-i18next hook](https://react.i18next.com/latest/usetranslation-hook) 及 [i18next configuration](https://www.i18next.com/overview/configuration-options)（2026-09-09 查證）；版本及授權見 lockfile／THIRD_PARTY。

## 安全與部署界線

無遙測、網路帳戶、雲端牌譜、客戶端接入、HUD／RTA／MDA、公開發布或自動更新。前端 raw 文本由 React escape；動態 SQL 限定欄位白名單與參數，CSV 文本欄位阻止公式注入。ZIP 直接串流讀取，不接受 archive 路徑寫入。

開發 HTTP server 只綁 loopback，驗證 Origin、JSON content type、POST `/api`；正式桌面不啟動該 server。Tauri CSP 僅允許本地資源與 IPC，dialog 權限為 open/save。native command 參數由 Rust serde 驗證；錯誤傳回 UI。

目前 raw log、notes 都未加密；本機 OS 檔案存取權／磁碟加密由使用者電腦管理。本版個人使用，未做公開分發簽署／notarization。

## UI 審核修正（2026-09-09）

- `useRpc` 的結果帶請求 key，切換條件立即隱藏舊結果，過期回應不更新目前畫面；手牌游標分頁亦檢查 query key。
- `Filter.source_date` 對應 `hands.local_date`，供來源日期報表點入；`date_from`／`date_to` 保留指定時區的 timestamp 語義。初始化加入 `hand_source_date` 索引，不修改 parser／統計公式。
- `issues` 接受 optional `before`，回傳 `rows`、`total`、`next_cursor`。已有獨立 raw 的問題直接開啟 raw；其他問題依 job 的 profile 與 Hand ID 配對。
- 共用 `Dialog` 使用原生 modal/inert，另處理 Tab 循環、原觸發項焦點及使用者編輯／忙碌狀態的關閉防護。`Field` 以 label/id 與 `aria-describedby` 分開名稱及提示。
- 所有儲存偏好均可在 localStorage 受阻時降級為本次 session 狀態。語言字典各 563 項；未知診斷保留原文。

返回頁面時先等待載入佔位內容被替換，再恢復記錄的主內容捲動位置；使用者開始捲動、點擊或鍵盤操作時停止自動還原。
