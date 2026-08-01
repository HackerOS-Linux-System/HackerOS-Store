import { createContext, useContext, createSignal, type JSX, type Accessor } from "solid-js";
import { translate, format, type Lang, type TranslationKey } from "../i18n";

interface I18nContextValue {
  lang: Accessor<Lang>;
  setLang: (l: Lang) => void;
  t: (key: TranslationKey | string, vars?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextValue>();

export function I18nProvider(props: { initial?: Lang; children: JSX.Element }) {
  const [lang, setLang] = createSignal<Lang>(props.initial ?? "en");
  const t = (key: string, vars?: Record<string, string | number>) => format(translate(lang(), key), vars);
  return (
    <I18nContext.Provider value={{ lang, setLang, t }}>
      {props.children}
    </I18nContext.Provider>
  );
}

export function useI18n(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n() must be used inside <I18nProvider>");
  return ctx;
}
