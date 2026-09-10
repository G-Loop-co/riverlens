import { useState } from "react";
import { useTranslation } from "react-i18next";
import { readTheme, selectTheme, supportedTheme, themes } from "../theme";

export function ThemeSelect() {
  const { t } = useTranslation();
  const [selected, setSelected] = useState(() =>
    supportedTheme(document.documentElement.dataset.theme ?? readTheme()),
  );
  return (
    <section className="panel theme-setting" aria-labelledby="theme-heading">
      <h2 id="theme-heading">{t("外觀主題")}</h2>
      <p>{t("即時套用並儲存於本機，下次開啟時保留您的選擇。")}</p>
      <fieldset className="theme-options">
        <legend className="sr-only">{t("外觀主題")}</legend>
        {themes.map((theme) => (
          <label className="theme-option" key={theme.id}>
            <input
              type="radio"
              name="theme"
              value={theme.id}
              checked={selected === theme.id}
              onChange={() => {
                selectTheme(theme.id);
                setSelected(theme.id);
              }}
            />
            <span
              className="theme-preview"
              data-theme={theme.id}
              aria-hidden="true"
            >
              <span className="theme-preview-sidebar" />
              <span className="theme-preview-content">
                <i />
                <i />
                <i />
              </span>
            </span>
            <span className="theme-option-title">{t(theme.label)}</span>
            <span className="theme-option-description">
              {t(theme.description)}
            </span>
          </label>
        ))}
      </fieldset>
    </section>
  );
}
