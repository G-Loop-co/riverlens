# 1,000萬手驗收：第一次資料生成

狀態：FAIL，尚未進入資料匯入。

合成 ZIP 的 population.txt 超過 ZIP 32-bit size 上限，ZipWriter 回報 `Large file option has not been set`。此結果不構成引擎效能驗收。

修正：ZIP writer 明確啟用 ZIP64 (`large_file(true)`)。第二次驗收使用獨立目錄，完整重新產生與匯入；原始失敗資料保留於 `/private/tmp/riverlens-mda-bench-10m-1/`。
