import i18n from 'i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import { initReactI18next } from 'react-i18next'

import en from './locales/en'
import zh from './locales/zh'

export const SUPPORTED_LANGUAGES = ['zh', 'en'] as const
export type AppLanguage = (typeof SUPPORTED_LANGUAGES)[number]

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      zh: { translation: zh },
      en: { translation: en },
    },
    fallbackLng: 'zh',
    supportedLngs: [...SUPPORTED_LANGUAGES],
    interpolation: { escapeValue: false },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
    },
  })

/** 归一化为应用支持的语言，用于选择 mock 数据中的本地化文案。 */
export function appLanguage(lang: string): AppLanguage {
  return lang.startsWith('zh') ? 'zh' : 'en'
}

export default i18n
