import { useTranslation } from "react-i18next";
import { changeLanguage, languages, supportedLanguage } from "../i18n";

export function LanguageSelect() {
  const { t, i18n } = useTranslation();
  return (
    <select
      className="language-select"
      aria-label={t("介面語言")}
      value={supportedLanguage(i18n.language)}
      onChange={(event) =>
        void changeLanguage(supportedLanguage(event.target.value))
      }
    >
      {languages.map(({ code, label }) => (
        <option key={code} value={code} lang={code}>
          {label}
        </option>
      ))}
    </select>
  );
}
