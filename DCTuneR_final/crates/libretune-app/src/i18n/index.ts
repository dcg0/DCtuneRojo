/**
 * i18n initialization for DC Tune Studio (fork of LibreTune).
 *
 * Importing this module side-effect once from `main.tsx` configures
 * `i18next` with all bundled locale resources and wires it into React
 * via `react-i18next`. After that, components use `useTranslation()`.
 *
 * Conventions:
 * - English (`en`) is the source language and the fallback.
 * - Spanish (`es`) is the primary UI language for DC Tune Studio.
 * - Namespaces map to UI surfaces (`common`, `menu`, `dialog`, etc.).
 *   Keep keys short; group by feature; never inline raw English in JSX
 *   for new code (use `t('namespace.key')`).
 * - INI-derived strings (menu items from ECU INI files, channel names,
 *   table titles) are passed through verbatim — they are content, not
 *   chrome, and translating them belongs in the INI itself.
 *
 * Adding a new language:
 *   1. Copy `locales/en/` to `locales/<code>/`.
 *   2. Translate every value (keys must remain identical).
 *   3. Register in the `resources` map below.
 *   4. Add it to the picker in `SettingsDialog` / languages.ts.
 */

import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

import enCommon from './locales/en/common.json';
import enMenu from './locales/en/menu.json';
import enDialog from './locales/en/dialog.json';
import enErrors from './locales/en/errors.json';

import esCommon from './locales/es/common.json';
import esMenu from './locales/es/menu.json';
import esDialog from './locales/es/dialog.json';
import esErrors from './locales/es/errors.json';

import ptBRCommon from './locales/pt-BR/common.json';
import ptBRMenu from './locales/pt-BR/menu.json';
import ptBRDialog from './locales/pt-BR/dialog.json';
import ptBRErrors from './locales/pt-BR/errors.json';

import huHUCommon from './locales/hu-HU/common.json';
import huHUMenu from './locales/hu-HU/menu.json';
import huHUDialog from './locales/hu-HU/dialog.json';
import huHUErrors from './locales/hu-HU/errors.json';

export {
  SUPPORTED_LANGUAGES,
  LANGUAGE_STORAGE_KEY,
  type SupportedLanguageCode,
} from './languages';
import { LANGUAGE_STORAGE_KEY } from './languages';

const resources = {
  en: {
    common: enCommon,
    menu: enMenu,
    dialog: enDialog,
    errors: enErrors,
  },
  es: {
    common: esCommon,
    menu: esMenu,
    dialog: esDialog,
    errors: esErrors,
  },
  'pt-BR': {
    common: ptBRCommon,
    menu: ptBRMenu,
    dialog: ptBRDialog,
    errors: ptBRErrors,
  },
  'hu-HU': {
    common: huHUCommon,
    menu: huHUMenu,
    dialog: huHUDialog,
    errors: huHUErrors,
  },
} as const;

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    supportedLngs: ['es', 'en', 'pt-BR', 'hu-HU'],
    load: 'currentOnly',
    defaultNS: 'common',
    ns: ['common', 'menu', 'dialog', 'errors'],
    interpolation: {
      escapeValue: false, // React already escapes
    },
    detection: {
      order: ['querystring', 'localStorage', 'navigator'],
      lookupLocalStorage: LANGUAGE_STORAGE_KEY,
      lookupQuerystring: 'lang',
      caches: ['localStorage'],
    },
    returnNull: false,
  });

export default i18n;
