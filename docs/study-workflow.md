# RiverLens 0.2.0 學習工作台

## 行為及資料界線

- Spot Explorer 建立 Hero 行動前的快照。公共牌、行動線、底池類型及對手均取決策當時值；不以最終盈虧、最終底池類型或攤牌結果篩選機會分母。翻牌後只收錄從翻牌開始單挑的手牌。
- 單手多次決策可增加機會數；手牌淨結果按不重複 hand 計算。Bet 顯示當次下注額；Raise 顯示本街加注後總額，均按該手 BB 換算。
- Leak Finder 支援自訂頻率區間與依據。GTO 目標依實際組合分布加權。預設 100 次機會後才有排序優先度；差距是描述性指標，不是顯著性檢定或 EV 損失。
- Nexus adapter 僅接受 `Cash6mGeneral_6mNL10R25`、100bb 的既有 manifest 格式。檔案需在 manifest 目錄內，拒絕路徑逃逸、重複行動、缺檔、非法權重及缺少 parent range。策略包按原始內容雜湊固定版本。
- 子行動權重除以 Hero 前一次行動的累積抵達權重；只有 Fold 合法時才補出 Fold 餘額。不把超出抵達權重的總和強行正規化。策略來源需自行提供，不隨原始碼或安裝包分發。
- GTO 統計要求 Hero 位置、完整前序行動／注碼、六人、六個起始籌碼均為 100bb、USD 0.05/0.10、無特殊下注規則及可用組合頻率。當前選擇 off-tree 注碼仍記為該匹配節點的偏差；前序路徑不符則只供參考。抽水模型無法由牌譜證明完全相同，UI 明示所選理論模型界線。
- Trainer 題目 JSON 在提交前不包含原本行動、來源筆記或答案頻率。策略包沒有 decision EV。正頻率行動表示可出現在混合策略，不等於每次都應選它。翻後使用自行複盤。
- 練習提交按 session／index 冪等；評級後安排 1/3/7/14/30 天。穩定參照包含 profile、Hand ID、action sequence、決策版本；重建 row ID 後重新定位回放。不可用題目可跳過。

## 儲存及遷移

Schema 3 增加 `study_decisions`、`study_indexed`、`study_items`、`study_packs`、`study_cards`、`study_sessions`、`study_attempts`。決策衍生版本為 `hero-decisions/2`；舊版本分批重新索引。既有帳本、原文、筆記及統計不被衍生索引覆蓋。

背景工作每批 50 手，共用既有 writer mutex；暫停在批次邊界生效。讀查詢使用 SQLite snapshot，排除尚未完成當前版本索引的手牌。導入原始牌譜時在同一交易建立新決策。

GTO 查詢先在精簡索引欄位分組，再解析每組資料；不逐手解析完整 JSON。探索彙總先按 hand 合併機會及行動數，再計一次該手淨結果。完整快照只為目前清單頁載入。

升級已存在的資料庫先保存 `before-study-*.db`。還原接受 schema 1/2/3，在 staging migration 後替換；完整備份包含所有學習資料。重算保留使用者資料表，再建立新的 derived rows。來源包、題目及嘗試紀錄的舊版本保持原樣。


策略包不隨原始碼分發。沒有 live HUD、RTA、MDA、postflop Solver 或 decision EV。
