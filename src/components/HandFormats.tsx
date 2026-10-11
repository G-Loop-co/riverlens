import { useTranslation } from "react-i18next";
import catalog from "../hand-history-formats.json";

export const importRooms = catalog.formats
  .filter((format) => format.status !== "research")
  .flatMap((format) =>
    format.rooms.map((room) => ({
      room,
      converted: format.status === "converted",
    })),
  );

export function HandFormats() {
  const { t } = useTranslation();
  return (
    <details className="panel">
      <summary>{t("支援格式與匯出指南")}</summary>
      <p className="muted">
        {t(
          "支援英文 NLHE Cash 及指定格式的 MTT／錦標賽，2–10 席、UTF-8。轉換格式必須是完整 PokerStars 牌譜；各牌室錦標賽支援範圍分開列出。",
        )}
      </p>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th>{t("品牌")}</th>
              <th>{t("格式")}</th>
              <th>{t("支援狀態")}</th>
            </tr>
          </thead>
          <tbody>
            {catalog.formats.map((format) => (
              <tr key={format.id}>
                <td>
                  <a href={format.guide} target="_blank" rel="noreferrer">
                    {format.label}
                  </a>
                  <small>{format.rooms.join(" · ")}</small>
                </td>
                <td>
                  {format.files}
                  <small>
                    {t(
                      format.status === "research"
                        ? "已找到資料 · 尚未支援"
                        : format.status === "converted"
                          ? "完整 PokerStars Cash／錦標賽轉換檔"
                          : format.tournament
                            ? "Cash ＋ 錦標賽 · 樣本測試"
                            : "Cash · 錦標賽未支援",
                    )}
                  </small>
                </td>
                <td>
                  {t(
                    format.status === "native"
                      ? "原生格式 · 樣本測試"
                      : format.status === "converted"
                        ? "僅 PokerStars 轉換檔"
                        : "已找到資料 · 尚未支援",
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p className="muted">
        {t(
          "格式測試不代表所有客戶端版本均已驗收。錦標賽以 CHIPS 保存；未知變體、Omaha、Short Deck、特殊結算及非標準轉換仍會拒絕或隔離。",
        )}
      </p>
    </details>
  );
}
