import vi from './vi.json';
import en from './en.json';

export type Locale = 'vi' | 'en';
type Dict = Record<string, string>;

const dictionaries: Record<Locale, Dict> = { vi, en };

const state = $state<{ locale: Locale }>({ locale: 'vi' });

export function setLocale(locale: Locale): void {
  state.locale = locale;
}

export function getLocale(): Locale {
  return state.locale;
}

/** Translate `key`, replacing `{name}` params. Falls back to `vi`, then to the key itself. */
export function t(key: string, params?: Record<string, string | number>): string {
  const raw = dictionaries[state.locale][key] ?? dictionaries.vi[key];
  if (raw === undefined) {
    if (import.meta.env.DEV) console.warn(`[i18n] missing key: ${key}`);
    return key;
  }
  if (!params) return raw;
  return raw.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}
