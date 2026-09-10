# Poker 分析工具研究矩陣

查證：2026-09-08。研究以官方功能頁／文件及 Browser 查證為依據，涵蓋主要產品類別，並非全球工具完整名錄。版本欄為官方頁顯示的產品世代；無公開版本標示者不臆測。沒有購買所有產品逐項實測。

「已提供」指官方文件描述現有功能；「官方聲稱」指宣傳性能／AI 能力，未經本專案獨立驗證；「尚未推出」指官方明確列為未來產品。

| 產品／版本 | 官方來源 | 已提供的功能類別 | 官方聲稱／未來項目 | RiverLens 決策 |
|---|---|---|---|---|
| Hand2Note 4 | [Reports](https://hand2note.com/Help/Features/reports) | 報表、情境篩選、HUD、自訂統計、手牌檢視 | 大型資料分析效能屬供應商聲稱 | 採報表 drill-down、分母與情境；不做 live HUD／對手 MDA |
| PokerTracker 4 | [Feature list](https://pokertracker.com/content/pokertracker-4-feature-list) | Tracker、圖表、replayer、篩選、notes、equity graph | 不把產品宣傳等同本工具結果一致 | 採本機資料庫、可追溯原文、受限 EV |
| Holdem Manager 3 | [HM3](https://www.holdemmanager.com/hm3/) | 報表、HUD、牌譜回放、檢討流程 | 效能描述未獨立量測 | 採分組報表與複盤入口 |
| Poker Copilot 8 | [官方](https://pokercopilot.com/) | Mac／Windows tracker、HUD、圖表、leak finding | Leak 評估不能直接推論 GTO 最佳行動 | 採易讀回放與筆記 |
| DriveHUD 3 | [官方](https://drivehud.com/) | Tracker、HUD、報表、replayer、分析功能 | AI 教練／策略能力屬官方聲稱 | AI 不列 V1；禁止虛構策略分數 |
| Xeester | [官方](https://xeester.com/) | Session、HUD、統計、分析及回放 | TITAN 為未來方向；不視為已提供功能 | 採 session／位置視角 |
| PokerCraft | [Natural8 官方](https://www.natural8.com/en/features/pokercraft) | 個人牌局紀錄、匯出、位置／起手牌資料 | 匯出保留期與支援項目以客戶端現行 UI 為準 | 手動匯出是資料入口；內文時區由 profile 明確設定 |
| GTO Wizard | [HH analyzer](https://help.gtowizard.com/how-to-use-the-hand-history-analyzer/) | 匯入手牌、對照解答庫與分析 | 分析依賴支援遊戲、配置及 solver 庫 | Solver 對照另期；V1 無 GTO score |
| PioSOLVER | [Feature overview](https://piosolver.com/docs/feature_overview/) | 遊戲樹、equilibrium solving、node locking、分析 | 取決於模型、樹及運算資源 | 後續獨立 Solver 接口 |
| GTO+ | [官方](https://www.gtoplus.com/) | Postflop solving、樹、range 分析 | 不把 solver 理論結果套用成未求解牌局建議 | 後續 |
| Flopzilla | [官方](https://www.flopzilla.com/) | Range／board 命中與組合分析 | 非追蹤資料庫完整替代品 | V1 提供實際起手牌矩陣；完整 range lab 後續 |
| Equilab | [官方](https://www.pokerstrategy.com/poker-software-tools/equilab-holdem/) | 手牌／range equity 分析 | 非 decision EV | 參考 equity 工具界線 |
| ICMIZER | [官方](https://www.icmizer.com/icmizer/) | Tournament、ICM、push/fold 分析 | 依 tournament 模型 | MTT／ICM 後續 |
| HoldemResources Calculator | [Postflop docs](https://www.holdemresources.net/docs/postflop/) | Tournament／ICM／postflop 樹與求解 | 大樹性能未独立驗證 | 後續 |
| MonkerSolver | [官方](https://www.monkerware.com/solver.html) | Hold'em／Omaha、多玩家求解 | 運算資源需求依樹而異 | PLO／大型 Solver 後續 |

## 技術選型與授權

| 元件 | 採用方式／理由 | 來源 |
|---|---|---|
| Tauri 2 | 原生 WebView、Rust IPC、本機 app data；正式程式沒有 HTTP API listener | [官方](https://v2.tauri.app/start/) |
| rusqlite | SQLite WAL、prepared statements、transaction、backup API；無外部 DB 服務 | [官方](https://github.com/rusqlite/rusqlite) |
| rs_poker 5.1.0 | Apache-2.0；使用 Card／SevenCardAccum 排名，補齊牌力、tie、費用、future-board invariance 測試 | [原始碼](https://github.com/elliottneilclark/rs-poker) |
| GG Hand Analyzer | MIT；參考 GG 匯出格式研究，不搬浮點金額及整批載入設計 | [原始碼](https://github.com/LayorX/GGPoker-Hand-Analyzer) |
| React 19／TypeScript／Vite | View state、typed request union；資料與計算由 Rust 提供 | 版本鎖於 package-lock.json |
| Recharts／Phosphor／Geist | 圖表、圖示及本地打包字型；無外部字型請求 | 版本及授權見鎖檔／THIRD_PARTY.md |

## 政策與 EV 定義

GG／Natural8 只採手動匯出、個人 Hero、賽後離線分析；不接入遊戲客戶端、不提供 RTA、live HUD 或對手群體 MDA，亦無平台認證。[GG Security & Ecology Policy](https://ggpoker.com/network/security-ecology-policy/) 查證版本 20260313；平台政策可變，以上是產品範圍而非平台批准。

EV 曲線採「實際結果＋已完成適用牌局調整」，只對完整帳本、HU、單底池、單 runout、雙方底牌已知、無 Cashout／特殊結算的 all-in 牌局計算；不是完整 EV、GTO score 或 decision EV。[PokerTracker equity graph 定義](https://docs.pokertracker.com/pt4/tutorials/hand-analysis-and-tools/all-in-equity-graphs/)
