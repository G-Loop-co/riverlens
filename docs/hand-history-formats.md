# 牌室格式與公開來源

查核日期：2026-10-11。此功能為個人複盤的原始碼擴充，尚未包含於 v0.4.0 下載包。MDA 群體匯入仍只接受 GG 一般桌 USD NLH 6-max；個人牌譜格式表不擴大 MDA 支援範圍。

RiverLens 的匯入範圍由 GG 擴至 **15 個原生格式家族**。目錄記錄 **94 個牌室、網絡、歷史品牌及開放標準條目**，不代表 94 個互不相同的 parser，也不代表所有牌室目前仍營運。匯入中心及來源設定共用 [`src/hand-history-formats.json`](../src/hand-history-formats.json)，每項均附來源連結及支援狀態。

處理自己的已完成、英文 **NLHE cash**，以及下列 **7 個錦標賽格式家族**，2–10 個實體座位、UTF-8。Omaha、Short Deck、未知動作、缺少 Hero 手牌或結算不符會拒絕或隔離。部分 GG 特殊結算沿用既有能力及 EV 排除規則；本次不擴大其驗收範圍。

## MTT／錦標賽

| 格式 | 已實作範圍 | 取得方式及限制 |
| --- | --- | --- |
| GG／Natural8 | `Poker Hand #…: Tournament #…`、Level、千位逗號、ante、籌碼 action／summary | PokerCraft → My Tournaments → 選已完成賽事 → Download Hand Histories；[GG 官方指南](https://help.ggpoker.co.uk/article/My-Tournaments-tab---Frequently-Asked-Questions)。賽事標題中的保證獎池／美元數字不當作 buy-in。 |
| PokerStars | `Hand`／`Game`、Tournament ID、Level、buy-in 原文、ante／big blind ante、退注、side pots | [官方索取 tournament hand history](https://www.pokerstars.com/help/articles/hh-single-tournament/)。完整英文 NLHE 文件；mixed-game header 不支援。已辨識的 finish／cash bounty 通知不進入 chip ledger，原文仍保留。 |
| WPN modern | `Game Hand #… - … Tournament #…`、Level、summary-only 派彩、UTC | 使用本機英文 hand histories。Legacy `Game started at` 的錦標賽變體尚未支援，不能以 cash 匯入。 |
| Winamax | Tournament／buyIn／level、2 或 3 欄 ante／SB／BB、由 Table 提取 ID | 使用本機牌譜。部分歷史樣本把 fractional chips 的派彩取整，會因嚴格核數而拒收；不猜改 0.5 籌碼差額。 |
| 888／Pacific | 明確 `Tournament #` metadata、Cassava 英文 chip actions、ante、分次派彩 | 雖以 `$` 標示 betting amounts，仍按錦標賽籌碼保存。特殊 BLAST、地區數字格式及結算不符變體未驗收。 |
| iPoker XML | `<tournamentname>`／`<tournamentcode>`、Holdem NL、session metadata、逐 action 及 player bet／win 核對 | XML 未提供 scheduled blinds 時，只接受兩席非 all-in 的完整盲注；short／all-in blind 無法確立級別，拒收。ID 缺失時保留 name，不杜撰編號。 |
| Full Tilt（歷史） | 明確 Tournament ID／Table、數字 SB／BB、ante、`raises to`、`Uncalled bet of`、派彩 | 僅自己的封存英文格式。`K` 縮寫盲注及 Cashout tournament 變體不支援。 |

介面選擇「MTT／錦標賽」會切換至 `CHIPS`。進階篩選可輸入來源賽事編號及 `50/100` 等自訂盲注；Study workspace 亦有賽事篩選。資料庫新增 indexed `tournament_id`，既有 populated database 在首次升級時先建立備份；舊 hand payload 缺少 optional tournament metadata 仍可讀取。CSV 包含 `currency`、`game`、`tournament_id`。

`net` 是每手籌碼結果；bb／100 按該手 BB 換算。`Adjusted` 僅沿用已符合條件的 **All-in chip EV**，未知 hole cards、side pots、多方 all-in 等仍依原規則排除。沒有計算獎金、報名費盈利、ROI、ITM、ICM、PKO bounty EV 或 re-entry accounting；header buy-in 僅保存原文字串。Cash strategy pack 的 USD／盲注／深度限制保留，不能當作 MTT GTO。來源無法區分 MTT／SNG 時，統一標為 `Tournament`，不猜賽制。

7 份獨立撰寫的合成牌譜，逐一核對 Hero +1,010 chips、pot 2,010 chips；另測 big blind ante、退注、bounty 通知、multiway side pots、fee／派彩不符、舊 schema／payload、混合現金／籌碼報表、匯入去重、重建及原文匯出。這些測試不取代現行客戶端驗收。

公開語法依據：[FPDB tournament corpus](https://github.com/jejellyroll-fr/fpdb-3/tree/development/regression-test-files/tour)，commit `84213a3e6fa97ff0f0994ae593e08cb4262ef05a`；[OHH PokerStars tournament example](https://hh-specs.handhistory.org/examples/holdem-tournament-hand-pokerstars)。額外抽查 14 個公開歷史案例，10 個核數成功，4 個 mixed／取整／結算不符案例拒收；[metadata-only audit 紀錄](hand-history-tournament-audit.json)。iPoker 抽查使用首手 game 與原 session metadata 的 envelope。未將 AGPL 程式或牌譜文字納入 repository。

## 原生格式及取得方式

「原生」代表以下特定語法有公開或合成樣本測試；尚未逐一登入各牌室、驗收所有地區／客戶端／版本。網絡下的品牌只是來源設定標籤，真正語法由牌譜內容偵測。額外 iPoker／WPN skin 名稱依 [DriveHUD 官方網絡清單](https://drivehud.com/knowledge-base/what-poker-sites-does-drivehud-support/) 整理，包括歷史品牌；不代表已登入驗收每個 skin。

| 格式家族 | 牌室／品牌 | 格式與取得方式 | 本次證據及限制 |
| --- | --- | --- | --- |
| GG | Natural8、GGPoker、GGNetwork | PokerCraft 選取已完成牌局，下載 TXT／ZIP；[PT4 官方設定指南](https://docs.pokertracker.com/pt4/site-configuration/ggpoker-configuration-guide/) | 沿用 `Poker Hand #` GG USD 現金桌 parser；既有 GG 回歸測試。沒有新增 AoF、bomb pot 或全部特殊桌種保證。 |
| PokerStars | PokerStars；地區版需符合相同語法 | 開啟英文 hand history 儲存，匯入資料夾內 TXT；[牌室官方說明](https://www.pokerstars.com/help/articles/save-hand-histories/) | `PokerStars Hand #`／舊 `Game #`、cash／Zoom、完整 actions 與 SUMMARY；MIT 公開 showdown 樣本、合成退注／時區測試。 |
| WPN | Winning Poker Network、Americas Cardroom、Black Chip Poker、PokerKing、TruePoker | 儲存英文牌譜；[PT4 官方指南](https://docs.pokertracker.com/pt4/site-configuration/winning-poker-network-configuration-guide-wpn/) | 新 `Hand # … Holdem(No Limit)` 及舊 `Game started at`；BLITZ 表名分類、舊版增量 raise、新版 summary-only walk payout。UTF-16 舊檔須先轉 UTF-8。 |
| 888 / Pacific | 888poker、Pacific Poker | 開啟 hand history 儲存；[PT4 官方指南](https://docs.pokertracker.com/pt4/site-configuration/888-poker-configuration-guide/) | `#Game No :` 舊英文現金桌，bracketed raise 為增量；MIT 公開樣本。未聲稱支援所有新版格式。 |
| partypoker | partypoker、bwin Poker | 支援的客戶端可在 My Game → Export Hands 匯出；[PT4 官方指南](https://docs.pokertracker.com/pt4/site-configuration/partypoker-configuration-guide/) | `***** Hand History for Game` 舊英文現金桌、增量 raise；MIT 公開樣本。匯出如使用 `Hero`，來源設定亦須使用 `Hero`。 |
| Winamax | Winamax | 啟用本機 hand history；[PT4 官方指南](https://docs.pokertracker.com/pt4/site-configuration/winamax-configuration-guide/) | `Winamax Poker … CashGame … HandId`、EUR、淨 pot 加 fee、未跟注部分退回；MIT 公開 all-in 樣本。 |
| iPoker | iPoker、bet365、Betfair、Titan、William Hill、NetBet、Red Star | 啟用英文手牌儲存；[PT4 官方指南](https://docs.pokertracker.com/pt4/site-configuration/ipoker-configuration-guide/) | `<session><general>…<game gamecode>` XML；session metadata、序號排序、suit-first 卡牌、action type、player bet/win 核對；MIT GBP 公開樣本。 |
| CoinPoker | CoinPoker | PokerIntel → Game History → Download Hands，Lifetime／Custom，從電郵連結取得自己的檔案；[PT4 官方指南](https://docs.pokertracker.com/pt4/site-configuration/coinpoker-configuration-guide/) | 原生 `CoinPoker Hand #… NLH (₮SB/₮BB/₮ante)`，USDT；合成語法及 MSK 測試。舊 PokerStars 轉換檔走 PokerStars parser。[PT 官方論壇的原生樣本及 auto blind 問題](https://www.pokertracker.com/forums/viewtopic.php?f=58&p=381240&t=105606)。auto big blind 與 header BB 不符會拒收，絕不猜改金額。 |
| Ignition | Ignition、Bovada、Bodog | 從客戶端下載已完成牌譜；[PT4 官方指南](https://docs.pokertracker.com/pt4/site-configuration/ignition-configuration-guide-bovadabodog/) | `… Hand #… TBL#… HOLDEM No Limit` 及 Zone header、`[ME]` 映射至設定 Hero、退注／Hand result；合成匿名 HU 測試及私人本機公開歷史樣本驗證。未知附加結算仍隔離。 |
| Unibet | Unibet | 下載手牌歷史；[PT4 官方指南](https://docs.pokertracker.com/pt4/site-configuration/unibet-configuration-guide/) | `Game #… No Limit Hold'Em`／Banzai、EUR、保留原文的 gross payout 規則；合成 all-in fold 測試及私人本機公開歷史樣本驗證。`Preflop` 標記不會建立 flop board。 |
| Full Tilt（歷史） | Full Tilt Poker | 自己的封存 TXT；[MIT 公開樣本](https://github.com/HHSmithy/PokerHandHistoryParser/tree/master/HandHistories.Parser.UnitTests/SampleHandHistories/FullTilt) | `Full Tilt Poker Game #`、raises-to、wins pot、EST；公開 showdown 樣本。 |
| Cake / Revolution（歷史） | Cake Poker、Revolution Poker | 自己的封存 TXT；[FPDB 原始語法樣本](https://github.com/jejellyroll-fr/fpdb-3/tree/development/regression-test-files/cash/Cake) | `Hand#… NL Hold'em`、DEALER、comma cards、returns uncalled bet、帶冒號的 wins 派彩行；自行建立合成樣本及私人本機公開歷史樣本驗證，沒有把 AGPL parser 或 fixture 納入 repository。 |
| OnGame（歷史） | OnGame | 自己的封存 TXT；[MIT 公開樣本](https://github.com/HHSmithy/PokerHandHistoryParser/tree/master/HandHistories.Parser.UnitTests/SampleHandHistories/OnGame) | `***** History for hand`、CET、main/side pot payout、rake；公開 showdown 樣本。 |
| Merge（歷史） | Merge / Carbon Poker | 自己的封存 XML；[MIT 公開樣本](https://github.com/HHSmithy/PokerHandHistoryParser/tree/master/HandHistories.Parser.UnitTests/SampleHandHistories/Merge) | `<description>`＋`<game>`、0-based seat、RAISE target／ALL_IN increment、uncalled winner；公開 all-in 樣本。 |
| Microgaming / MPN（歷史） | Microgaming / MPN | 自己的封存 XML；[MIT 公開樣本](https://github.com/HHSmithy/PokerHandHistoryParser/tree/master/HandHistories.Parser.UnitTests/SampleHandHistories/MicroGaming) | `<Game hhversion="4">`、USD／EUR／GBP symbol、每席 endbalance 核對；公開 all-in 樣本。舊 text grammar 未實作。rake 屬性是比率，不當作現金額。 |

## 只接受已完成的 PokerStars 轉換檔

以下名稱可選作來源標籤，但 RiverLens **沒有實作其原生 capture 或 converter**。必須已持有包含盲注、所有動作、退注、payout、SUMMARY 及 Hero 手牌的英文 PokerStars cash TXT。僅選擇品牌不會將不相容的原文變成可解析資料。

| 家族 | 名稱 | 線上來源 | 限制 |
| --- | --- | --- | --- |
| Chico | Chico、BetOnline、SportsBetting、TigerGaming | [DriveHUD 官方支援清單](https://drivehud.com/knowledge-base/what-poker-sites-does-drivehud-support/) | Tracker 的原生／capture 支援不等於可直接匯出 PokerStars TXT；轉換檔須另行取得及通過本地核對。 |
| Club apps | PPPoker、PokerBros、UPoker、ClubGG、X-Poker、QQPK | [Hand2Note 官方牌室清單](https://hand2note.com/Help/supported-sites)、[PPPoker 官方整合指南](https://hand2note.freshdesk.com/support/solutions/articles/8000077735-pppoker) | 不提供遊戲客戶端連接、即時擷取或 HUD。QQPK 截圖上的 PokerStars 標籤僅作轉換格式線索。 |
| WPT Global | WPT Global | [牌室官方 QuintAce 複盤工具說明](https://wptglobal.com/poker/promotions/turn-your-leaks-into-wins) | 找到內建複盤工具；未確認公開、完整的原生匯出規格或 fixture。此處只表示已有標準 PokerStars 轉換檔可嘗試匯入，不代表已驗證轉換器或現行匯出服務。 |

## 找到來源、尚未實作

| 名稱 | 公開格式線索 | 狀態 |
| --- | --- | --- |
| Boss Media / IPN | [HHSmithy 原始 XML fixture](https://github.com/HHSmithy/PokerHandHistoryParser/tree/master/HandHistories.Parser.UnitTests/SampleHandHistories/BossMedia) | 尚未處理專用卡牌及動作編碼。 |
| Absolute Poker、UltimateBet、Everest、Everleaf、Entraction、PKR | [FPDB 原作者的支援矩陣與 converter](https://github.com/jejellyroll-fr/fpdb-3/blob/development/PARSER_SUPPORT.md)、[HHSmithy](https://github.com/HHSmithy/PokerHandHistoryParser) | 有歷史 parser／sample，尚未實作。 |
| PokerMaster、Seals with Clubs、KingsClub | [FPDB converter 原始碼](https://github.com/jejellyroll-fr/fpdb-3/tree/development/fpdb_3_legacy) | 只有格式研究線索，尚未實作或驗收；不代表現在仍可匯出。 |
| Adda52、Pocket52、PokerBaazi、The Spartan Poker、BlitzPoker／FTR Poker、WSOP.net、Intertops／Horizon、Juicy Stakes | [DriveHUD 官方牌室清單](https://drivehud.com/knowledge-base/what-poker-sites-does-drivehud-support/) | 有 tracker 支援線索，未取得可直接匯入 RiverLens 的原生規格及完整樣本。 |
| Hive Poker、Run It Once Poker、MPLPoker、SupremaPoker | [FPDB PokerStars converter](https://github.com/jejellyroll-fr/fpdb-3/blob/development/fpdb_3_legacy/PokerStarsToFpdb.py) | 開源 converter 有格式識別分支；RiverLens 尚未支援這些專用 header。 |
| PHH | [PHH 原作者規格及資料集](https://github.com/uoftcprg/phh-std)、[PokerKit](https://github.com/uoftcprg/pokerkit) | TOML 標準，尚未實作。 |
| Open Hand History（OHH） | [官方 JSON 儲存規格](https://hh-specs.handhistory.org/storage-format) | 尚未實作；不能直接採用以 f32 儲存金額的模型來取代精確 ledger。 |
| ACPC | [PokerKit 原作者 parser](https://github.com/uoftcprg/pokerkit) | 遊戲研究格式，尚未映射至現金桌會計。 |

## 匯入、金額及原文

- TXT、XML、含兩者的 ZIP 及資料夾均由既有匯入服務處理；副檔名不決定 parser，內容才決定語法。
- 不連線登入牌室、不購買 datamined histories、不下載私人帳戶資料。來源網站及使用者截圖的內容只作研究資料。
- 重用既有 Rust fixed-point ledger、統計、重複／衝突判斷、隔離及 rebuild 路徑。依不同牌室處理 raise target／increment、gross／net pot 及 uncalled return，不繞過保守核對。
- USD、EUR、GBP、CAD、USDT、CHIPS 分別保存；報表及矩陣禁止混合幣別／籌碼加總。介面預設 USD，可切換；CSV 每列帶單位。沒有自動匯率轉換。
- 明確 ET／UTC／EST／EDT／CET／CEST／MSK tag 優先；沒有時區 tag 時使用來源設定的 fallback。來源設定不是「香港顯示時區」。日期篩選以指定時區的 played_at 計算，避免來源 tag 與 profile 不同而錯用 local-date rollup。
- 原 TXT 內容保留於每手 raw。XML 多手檔拆成獨立 game envelope，保留必要 session／description metadata；rebuild 及 HH 匯出使用此可解析原文。來源檔案本身不變。XML 的外層 envelope 可能重組，並非逐 byte 複製整個 session。
- TXT 串流保留來源內部的 LF／CRLF；去重 fingerprint 沿用 CRLF→LF 正規化，換行不同的同一手不會新增或誤報衝突。Git fixtures 固定 LF，測試另生成 CRLF ZIP 及 LF 重匯檔，避免依賴 runner checkout 設定。
- 無獨立 cash rake 明細時，把 ledger 與 payout 差額標成 `Unspecified fee`，不猜成 rake；相應 all-in adjusted EV 仍依既有排除規則處理。
- XML DTD／外部 entity 拒絕；metadata 上限 64 KiB，單手 1 MiB。未知 XML action type、MPN endbalance 或 iPoker source bet 不符一律拒絕。
- Native format support 不代表 native app 驗收、牌室現行政策驗證、部署或正式發行。

## 測試與授權

[`crates/poker-core/tests/fixtures/hand-formats/provenance.json`](../crates/poker-core/tests/fixtures/hand-formats/provenance.json) 記錄 HHSmithy commit、原路徑、原檔 SHA-256、匿名化變更及 MIT 授權；同目錄附原 MIT notice。只替換公共 player alias、統一 UTF-8 並移除重複的 encoded alias metadata，動作與金額不變。

10 份 MIT 公開 fixture 涵蓋 PokerStars、Full Tilt、partypoker、888、Winamax、WPN legacy、iPoker、OnGame、Merge、MPN。其 Hero 淨結果分別獨立核對為 USD +282、+2.76、+0.09、-6；EUR -22.85；USD +109.90；GBP -3.73；USD +6.75、+206.60；EUR +0.59。公開樣本選用已知 cards 的玩家作測試 Hero，不代表原檔擁有者。新增合成測試涵蓋 WPN modern、CoinPoker、Cake、Unibet、Ignition、來源時區、walk、退注、dealer／位置、缺失結算及不可解析 variants。

整合測試涵蓋多手 XML metadata、停止邊界、損坏 XML／DTD／未知 action、混合 TXT／XML ZIP、UTF-8 buffered boundary、XML showdown metadata 不可提前變成 public reveal、再匯入去重、rebuild、raw 匯出，以及 mixed-currency rollup／direct report／matrix／CSV。沒有以合成樣本取代真實牌室客戶端驗收。

FPDB 為 AGPL，僅研究公開語法，未將其 parser 程式或 fixture 納入 RiverLens。XML 使用現成 `roxmltree` 與 `quick-xml` crate；既有金額與結算模型保留。

## 本機驗證紀錄（2026-10-11）

### v0.4.0 MDA 整合回歸

以 `origin/main` 的 `0d4c6d437b9d77e4aa026cbd9885bfa0b9e9f598` 建立 `feat/multi-room-mtt-formats` 整合分支，保留既有 MDA／AI 及 v0.4.0 文件。個人來源先正規化，再進入既有 ledger；群體 parser 維持原生 GG USD cash 路徑，不能藉個人格式擴充匯入 PokerStars、iPoker 或錦標賽。

- `cargo test -p poker-core -p riverlens-ai`：107 個測試通過；包含 17 個 Cash／MTT 格式測試、4 個 MDA 整合測試、資料集分享權限及 100k index／resume／cursor 測試。100k 所在測試套件耗時 206.14 秒，不代表吞吐量驗收。
- `npm test`：45 個測試通過；新增匿名 MDA 回放只讀群體資料、不讀取／編輯個人筆記及訓練佇列的回歸。`npm run build` 通過，保留原有 Vite 提示。
- `cargo clippy -p poker-core -p riverlens-ai -p riverlens --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 及 2 個簽章環境測試：通過。
- 首輪 Windows CI 發現 TXT 原文 CRLF 被既有 `lines()` 串流改為 LF；修正成保留換行的 `read_line()`，並補強 ZIP 原文、LF／CRLF 去重及重建／匯出回歸。修正後本機 39 個格式／MTT／MDA／匯入邊界測試及 clippy 通過；初次失敗保留於 CI 紀錄。
- 上述為整合後原始碼測試；沒有重跑 1,000 萬手效能或全部原生 UI 驗收，也沒有把先前原生 App 證據視作新整合 binary 的驗收。

### v0.3.2 基底的牌譜擴充測試

- `cargo test -p poker-core -p riverlens-ai`：101 個測試通過，包含 6 個 MTT 整合測試及 100k index／resume／cursor 測試（216.38 秒，debug correctness 測試，不代表吞吐量驗收）。
- 後續 All-in suffix 缺失推導及金額防護修正：100 個回歸測試通過（略過已通過的 100k）；最後嚴格金額 token 修正再跑 16 個 Cash／MTT 格式測試，全部通過。7 個合成格式的 heads-up All-in 均進入 chip-equity pending，multiway side pots 則保留排除。
- `cargo clippy -p poker-core -p riverlens-ai -p riverlens --all-targets -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check`：通過。
- `npm test`：41 個測試通過；`npm run build`：通過。Vite 仍有原有 dynamic import／chunk size 提示。
- 公開 corpus 抽查：14 個案例，10 個有效、4 個拒收；屬語法探索，並非現行客戶端驗收。
- 開發分支 `feat/mtt-hand-histories` 從 `origin/main` 建立，保留前一階段多牌室 Cash 改動。沒有產生或發行新 installer，也沒有安裝或操作真實牌室帳戶。

### 原生 App 驗證

以下原生驗證使用 v0.3.2 基底加本次牌譜擴充，早於 v0.4.0 MDA 整合；不能作為整合後 App 的原生驗收紀錄。使用本機 ad-hoc 簽章的臨時 App，`cargo build --release -p riverlens --features tauri/custom-protocol` 內嵌正式前端，實際 origin 為 `tauri://localhost`。最終 binary SHA-256：`4021a9c40b8c9e87934404ea6cd4b06fbc87824ae08704e6bd32b377d0399d7c`。新隔離資料庫路徑以測試 bundle 的 `LSEnvironment RIVERLENS_DATA_DIR` 設定，並在原生設定介面核對；詳見 [Apple LSEnvironment 文件](https://developer.apple.com/documentation/bundleresources/information-property-list/lsenvironment)。

- 固定 14 份 FPDB 公開錦標賽歷史檔，合共 17 手：13 手有效、4 手拒收；13 手 chip ledger 與獨立來源核算一致。7 個支援格式各完成一次原生回放、最終 Hero 籌碼及籌碼帳本核對。iPoker corpus 包含一手 SNG，仍統一標記 `Tournament`。
- 同一 MTT 來源重匯：0 新增、13 重複、0 衝突、同樣 4 手拒收；賽事編號篩選及重啟保存通過。CHIPS 統計不含 Cash；未計算 ROI、ICM 或 bounty EV。
- 固定 13 份公開 Cash 歷史檔：10 份 MIT fixture，加 Ignition、Unibet、Cake 私人 FPDB 樣本；最終 13／13 有效，原文、幣別及獨立 Hero 結果核對通過。初跑發現 Unibet `Preflop` 與 Cake 派彩冒號邊界，補合成回歸、修正後重建並以同批樣本重新驗證；初次失敗另留紀錄。修正後 16 個 Cash／MTT 格式測試及 clippy／fmt 通過。
- 既有 Cash／Rush 原文亦在隔離 App 重匯並逐手核對；私人資料與證據只保留於 gitignored `.local`。

上述證明本機原生流程及已知歷史格式相容；不是現行牌室客戶端匯出認證。CoinPoker 原生及 WPN modern Cash 仍欠本次原生公開真實樣本證據；converted-only 牌室沒有因此取得原生支援。公開測試來源 profile 手動使用共同標籤，不能視作自動品牌識別。AGPL 原始檔僅在私人測試目錄保留，未納入 repository、installer 或發布內容。沒有安裝、notarize 或發行新版。
