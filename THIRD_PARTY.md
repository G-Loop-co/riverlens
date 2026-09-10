# 第三方元件

RiverLens 使用鎖檔指定版本；完整 transitive dependencies 以 Cargo.lock、package-lock.json 為準。無抄錄商業軟體 UI 資產、牌譜資料或封閉程式碼。

| 元件 | 授權 | 用途 |
|---|---|---|
| Tauri 2、tauri plugins | MIT OR Apache-2.0 | Desktop shell／dialog |
| rusqlite | MIT | SQLite binding |
| SQLite | Public domain | 嵌入式資料庫 |
| rs_poker | Apache-2.0 | Card／SevenCardAccum 排名 |
| serde、serde_json | MIT OR Apache-2.0 | Typed serialization |
| chrono、chrono-tz | MIT OR Apache-2.0 | 時區與時間 |
| regex、flate2、sha2 | MIT／Apache-2.0 系列；依 crate manifest | Parser、壓縮、hash |
| zip | MIT | ZIP 串流讀取 |
| React、Vite、TypeScript、Tailwind CSS | MIT | 介面與建置 |
| Recharts | MIT | 盈虧圖表 |
| i18next、react-i18next | MIT | 本機三語資源、React 語言切換及單複數 |
| opencc-js | MIT AND Apache-2.0 | 建置工具：產生簡體中文字典；不轉換使用者資料、不包含於 runtime |
| Phosphor icons | MIT | 介面圖示 |
| Geist／Geist Mono | SIL Open Font License 1.1 | 本機打包字型 |
| Testing Library React、jsdom | MIT | 開發測試：互動、查詢競態、欄位可及性回歸；不包含於 runtime |
| axe-core | MPL-2.0 | 開發驗證：瀏覽器可及性掃描；不包含於 runtime |

GG Hand Analyzer（LayorX，MIT）僅作格式研究參考；核心 parser 以 exact units 與 streaming 設計獨立實作。原始碼鏈接、查證日期與決策見 docs/research-matrix.md。

正式再分發前應附完整鎖定版本的第三方 LICENSE／NOTICE 集合；目前交付為個人本機開發包。
