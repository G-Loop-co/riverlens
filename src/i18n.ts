import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import traditional from "./locales/zh-TW.json";
import simplified from "./locales/zh-CN.json";
import english from "./locales/en.json";

export const LANGUAGE_KEY = "riverlens-language";
export const languages = [
  { code: "zh-TW", label: "繁體中文" },
  { code: "zh-CN", label: "简体中文" },
  { code: "en", label: "English" },
] as const;
export type Language = (typeof languages)[number]["code"];

export function supportedLanguage(value: string | null): Language {
  return languages.find((language) => language.code === value)?.code ?? "zh-TW";
}

function initialLanguage(): Language {
  if (typeof window === "undefined") return "zh-TW";
  try {
    return supportedLanguage(localStorage.getItem(LANGUAGE_KEY));
  } catch {
    return "zh-TW";
  }
}

void i18n.use(initReactI18next).init({
  resources: {
    "zh-TW": { translation: traditional },
    "zh-CN": { translation: simplified },
    en: { translation: english },
  },
  lng: initialLanguage(),
  fallbackLng: "zh-TW",
  supportedLngs: languages.map((language) => language.code),
  load: "currentOnly",
  keySeparator: false,
  nsSeparator: false,
  initAsync: false,
  interpolation: { escapeValue: false }, // React escapes rendered text.
  react: { useSuspense: false },
});

function updateDocument() {
  if (typeof document === "undefined") return;
  document.documentElement.lang = supportedLanguage(i18n.language);
  document.title = i18n.t("RiverLens — 牌譜分析工作台");
}
i18n.on("languageChanged", updateDocument);
updateDocument();

export function changeLanguage(language: Language) {
  try {
    localStorage.setItem(LANGUAGE_KEY, language);
  } catch {
    // The current session can still change language if storage is unavailable.
  }
  return i18n.changeLanguage(language);
}

export default i18n;
