# MDA 群體分析 v1

範圍：GG 一般桌、USD、NLH 6-max。資料集獨立於個人 SQLite 與學習庫；未知底牌不推測。翻後候選預設 HU，各級別分開。

流程：匯入 TXT／ZIP／資料夾 → 篩選情境 → 合法決策機會統計 → AI 調整建議 → 匿名手牌回放 → 既有學習草稿審閱。

## 來源與相容性

首版以標準 GG grammar 共用 RiverLens 整數金額、牌面與結算驗證。HHDealer 的 GG Regular／PT、HM、H2N4 是格式目標，目前沒有供應商真實樣本。相容性須經真實匯入、統計及回放確認；不以合成測試代替。

參考：[Hand2Note Multi-player Reports](https://www.hand2note.com/Blog/News/multi-player-report-in-hand2note4)、[DuckDB Rust Appender](https://duckdb.org/docs/current/clients/rust/data_import)、[HHDealer 格式](https://www.hhdealer.com/buy.php)。Appender 明確 flush 成功後才 commit；每批資料、增量彙總和 checkpoint 在同一交易。DuckDB 預設 memory_limit=2GiB、threads=4；buffer limit 不等同程序 RSS。

## 指標與證據約定

百分比 = 總命中次數 ÷ 總合法機會，禁止平均玩家百分比。只使用決策前可見的牌面、底池、存活人數和剩餘有效籌碼。全桌玩家有合法決策才產生機會；walk、全下後、未重新開放的加注不算。

RFI、盲位面對單次 open 的 fold／call／raise、3-bet、面對 3-bet、各街延續下注及其 fold／call／raise 回應、check-raise、前街全桌 check 後下注。特殊 straddle／不完整牌譜隔離，不混入首版分母。

| 指標 | 合法機會分母 | 分子 |
|---|---|---|
| `rfi` | 無人 limp／raise，玩家可合法加注；排除 BB walk | raise |
| `three_bet` | 面對一次 open 且可合法再加注；包含 squeeze | raise |
| `blind_fold/call/raise` | SB／BB 面對一次 open，沒有 cold caller，且需跟注；三種回應共用決策分母 | 對應 fold／call／raise |
| `fold/call/raise_three_bet` | 原 open 者面對一次 3-bet，且需跟注 | 對應回應 |
| `cbet` | 前街最後進攻者，本街首次可下注的決策，尚未有人下注 | bet |
| `fold/call/raise_cbet` | 面對本街 C-bet，尚未出現中間 raise | 對應回應 |
| `check_raise` | 本街曾 check，現需跟注而且可合法 raise | raise |
| `bet_checked_through` | 前一翻後街所有可決策玩家 check；本街首次可下注決策，尚未有人 bet | bet |
| `fold/call/raise_bet` | 面對本街首次 bet，尚未出現中間 raise | 對應回應 |

預設 HU 表示該翻後街開始時有兩名未 fold 玩家；多人街道有人先 fold 不會改列 HU。同一玩家在不同決策可各有機會；回應選擇率以共同 fold／call／raise 決策分母，不能誤讀成只計可加注者的策略頻率。有效籌碼為決策當刻自身剩餘與最大未 fold 對手剩餘的較小值，換算 BB；HU 為雙方剩餘較小值。面對下注比例保存原下注時 bet／pot，raise 另以額外加注／跟注後底池計；不以玩家稍後選定尺寸篩選其自身機會。

日期按資料集指定內文時區的來源日界線篩選。各 BB 級別及各街始終分開；rollups 保存總數而非比率。精細有效籌碼、尺寸、paired／high-card 條件查原始決策列。當刻牌面不使用後續 runout；未知底牌不填補。前一街全桌 check 的指標涵蓋 turn／river，不把未下注翻前自動算作 check-through。

候選預設至少 200 個機會；附描述性 95% Wilson 區間、時間分段、攤牌底牌覆蓋率與選擇偏差。沒有外部基準的結果明確是研究假設，不能聲稱偏離 GTO。pure bluff 的 B/(P+B) 只適用 HU river、零攤牌 equity、對手只 fold／call、尺寸固定等已列明假設；不代表 solver EV。

分時段按命中樣本的來源日期範圍切成前後兩段，列分母、分子及區間是否重疊；任一時段沒有機會時標示證據不足。區間重疊不證明穩定，同桌／同玩家的重複決策也非獨立樣本。攤牌底牌覆蓋 = 有可見底牌的攤牌存活玩家決策 ÷ 攤牌存活玩家決策；不是完整群體 range 覆蓋。

## 資料集、工具與版本

個人 SQLite 維持原路徑；其旁邊的 `mda/` 每個資料集一個 `.duckdb`。資料集保存 GG 網絡、來源／牌池、格式、內文時區、USD、parser／stats schema 版本。去重以網絡＋原始 Hand ID；payload 壓縮保存解析結果及原文，同 ID 內容不同隔離且不覆蓋。續傳驗證來源路徑、大小、修改時間及 ZIP member CRC，來源需保持穩定。TXT／ZIP 單手及單行上限 1 MiB；資料夾／ZIP 不解壓寫入任意位置。

`get_mda_catalog` 讀指定資料集目前版本及指標；其餘 `query_mda_stats`、`find_mda_candidates`、`find_mda_spots`、`get_mda_hand` 均要求 `dataset_id`＋`version`。頁面最多 100；統計 cursor 上限 100,000 offset，手牌以 opaque hand key／seq 分頁。不接受任意 SQL、玩家姓名搜尋或原文外傳。匿名完整回放是事後證據；指定 seq 的投影清除未來牌面、底牌、結果及當刻尚未形成的 pot type。

獨立備份需先停匯入，checkpoint 後以 DuckDB [COPY FROM DATABASE](https://duckdb.org/docs/current/sql/statements/copy#copy-from-database--to) 複製完整 schema／sequence／資料到私有 staging 資料庫，detach 釋放檔案鎖，再原子保存。外部 SQL 檔案存取維持關閉，只預先允許該資料集固定的備份 staging 檔；避免 Windows 開啟中的資料庫鎖使 OS 檔案複製失敗。還原先驗證資料庫再建立新 ID／epoch，雲端及 MCP 權限關閉；不覆蓋個人庫。各資料集 DuckDB instance 的 buffer limit 是 2 GiB，並非多資料集同時運作的全程序硬上限；1,000 萬手驗收量測單資料集全程序 RSS。

AI 報告必須包含：適用情境、分子／分母、調整方向及尺寸、理由、例外、匿名手牌證據。版本變更停止分析。雲端與 MCP 分別按資料集啟用；只分享彙總及有界匿名案例。來源原文不是 AI 指令。

MDA 本機學習草稿在撤回分享後仍可檢視、編輯、確認及查看已保存的匿名證據；重新發出 AI 分析或分類須另有雲端權限。本機 library RPC 不暴露為 MCP 工具。

## 平台限制

GG 現行 [Security & Ecology Policy](https://ggpoker.com/network/security-ecology-policy/) 第 7／19 條限制收集他人牌譜及系統化收集／分享 MDA，可能導致停權、封禁或資金沒收。RiverLens 提供離線研究功能，不代表平台授權。不得將此功能文件當作 GG 合規許可。

## 驗證狀態

分母、未知底牌、去重與內容衝突、中斷續傳、增量彙總、備份還原、決策前資訊及 Coach/MCP 共用結果已有自動化測試。完整效能、平台打包與獨立外部驗收見 [MDA 驗證記錄](validation/mda-population-ai.md)。真實 provider、外部 MCP 客戶端、HHDealer 樣本分別記錄。
