# MDA 群體分析與 AI 驗證 — 2026-10-11

來源分支：`feat/mda-population-ai`，獨立 worktree，基於更新後的 `origin/main` `3f4155e69d84d6fa0a3d3e08688c7eaa9fc6c5e8`。原 checkout 保留。首版範圍：GG Regular、USD、NLH 6-max；所有資料為合成資料。

## 本機正確性與回歸

| 項目 | 結果與範圍 |
|---|---|
| Rust core | 原個人 parser／帳本／學習回歸通過；MDA 四項整合測試包含 256 個合法場景、無 Hero／未知牌、walk、短 all-in 不重開加注、決策前牌面及剩餘有效籌碼 |
| MDA 儲存／匯入 | 同交易 checkpoint、跨批／跨檔去重、衝突保留、ZIP 檔案中段續傳、增量彙總一致、備份還原、新 epoch 及關閉分享權限 |
| AI／IPC | 13 項 Rust AI 測試通過；MDA 本機／socket 共用結果精確一致、各自分享權限、匿名手牌、報告草稿及撤權拒絕；JSON 浮點採 float_roundtrip |
| 前端 | 40 項測試／8 個檔案通過；MDA 預設分享關閉、版本綁定、手牌回放使用 MDA opaque key；撤回分享後仍能在本機閱覽／確認草稿及開啟匿名證據；個人 AI／學習／三語／主題回歸通過 |
| 前端 build | 通過；既有大 chunk 與 dialog 混合匯入警告仍在 |
| Rustfmt／Clippy | 全 workspace format；core／AI／desktop all-targets Clippy `-D warnings` 通過 |
| 本機 Apple Silicon package | `.app` 成功建立（本機 `--no-sign` 驗證包）；不代表簽章或 notarization 驗收 |
| 打包 MCP protocol smoke | `initialize`／23 個 schemas／disabled access denial 通過；不是外部模型客戶端實測 |

MDA tests 的部分底牌缺失、多人首人 fold、短 all-in、檔案中段續傳及跨 ZIP member 重複均有專用斷言。既有 opt-in 私人資料測試沒有提供私人 archive，不宣稱私人資料驗收。

瀏覽器連到隔離 Rust core，10,000 手合成資料匯入完成，個人資料仍為 0 手。實際操作 HU flop `fold_cbet`：$0.02 級別顯示 418／1,254，33.3%，Wilson 30.8–36.0%；回查帶回 BB、flop、級別條件，匿名回放顯示 Seat 1–6、三張 flop 與未知牌背，沒有 Hero。分享預設關閉，沒有呼叫供應商。[實際畫面](mda-stats.jpg)。

## 1,000 萬手效能

狀態：RUNNING。完整重跑使用 ZIP64 合成 GG 牌譜及全新資料集；不把途中進度或 10,000 手小測試當作完整驗收。測量二進位來源為 `d5bbd2b`，JSON 保存完整核心來源 hash。後續修正本機草稿控制、UI、回放 DTO 的 flop 人數及跨平台備份（私有 staging 白名單／資料庫複製）；parser／facts／Appender／統計 SQL／匯入路徑及 memory_limit／threads 均沒有改動。這些修正以功能測試另驗，本次不是完整桌面程序或 1,000 萬手備份時間測量。

硬體：Apple M3 Pro、11 logical threads、18 GiB RAM、macOS aarch64。DuckDB crate `1.10506.0`；buffer memory limit `2GiB`，threads `4`。量測 parser、ZIP streaming、Appender、去重、彙總及 checkpoint，全程序 `getrusage` 峰值 RSS。每類情境各 21 次查詢，報 p50／p95／max；暖 OS cache，排除 UI／IPC 與真實供應商／AI 延遲。

重現（一般 Rust 環境）：

```sh
cargo run --release -p poker-core --bin mda-bench -- 10000000 /tmp/riverlens-mda-bench-new docs/validation/mda-bench-10m.json
```

需要全新輸出目錄與足夠磁碟。`RIVERLENS_MDA_BENCH_INPUT` 可重用完整已生成 ZIP；需記錄此情況，generation 秒數代表連結／複製耗時而非生成耗時。runner 記錄核心來源 SHA-256，資料唯一手數、隔離／衝突數、硬體、匯入秒數、ZIP／資料庫用量及效能閘門。

保留先前 [ZIP32 超限失敗](mda-10m-attempt-1.md) 及 [來源修正後中止舊引擎測試](mda-10m-attempt-2.md)；兩者都不是 PASS。`mda-bench-10k.json` 是早期小樣本探索，非最終 1,000 萬手驗收。

## 外部及平台驗收

| 閘門 | 狀態 |
|---|---|
| GitHub CI：macOS ARM／macOS Intel／Windows 打包 | PENDING；以分支來源 SHA 與各 matrix job 結果記錄 |
| Intel／Windows 本機啟動與憑證／IPC | NOT VERIFIED；cross-build 或 CI 打包不代表使用者裝置實測 |
| 內建真實 provider | NOT VERIFIED；沒有供應商 key，未發出付費分析請求 |
| 外部 MCP 模型客戶端 MDA 分析 | NOT VERIFIED；自動化 bridge／protocol smoke 與真實模型客戶端分開 |
| HHDealer GG Regular PT／HM／H2N4 | NOT VERIFIED；沒有真實檔案，待匯入、統計、回放測試 |
| Solver／node-lock | 不在首版；研究假設及 pure-bluff 算式不當作 GTO／decision EV |

GG SEP 第 7／19 條限制他人牌譜收集與 MDA 分享；本機測試及功能可用不代表平台批准。功能文件保留停權、封禁與資金沒收限制。

Windows 初次 CI `38119201078` 的備份整合測試失敗：開啟中的 DuckDB 有 write lock，直接 OS copy 回報 error 32。改用官方資料庫複製、detach 後原子保存，保留該次 FAIL；續傳／備份測試增加覆寫備份、特殊字元路徑、隔離原文與 sequence 還原斷言。三平台以修正後 SHA 重驗。

本機 Mac 在原生介面覆核前鎖定；已完成瀏覽器對隔離 Rust core 的操作及原生 package／protocol smoke，原生視窗操作仍為 NOT VERIFIED。
