/**
 * Pure-data language constants, safe to import without side effects.
 *
 * Importing from here does NOT load i18next. Components that only need to
 * render a language picker or read the localStorage key should import this
 * module instead of `../i18n` (which triggers init).
 */

export const SUPPORTED_LANGUAGES = [
  { code: 'es', label: 'Español' },
  { code: 'en', label: 'English' },
  { code: 'pt-BR', label: 'Português (Brasil)' },
  { code: 'hu-HU', label: 'Magyar' },
] as const;

export type SupportedLanguageCode = (typeof SUPPORTED_LANGUAGES)[number]['code'];

export const LANGUAGE_STORAGE_KEY = 'libretune.lang';
