import { buildLocaleMessages, createMessageCompiler } from '@modrinth/ui'
import { createI18n } from 'vue-i18n'
import chineseApp from './locales/zh-CN/index.json'
import chineseUi from '@modrinth/ui/src/locales/zh-CN/index.json'

const i18n = createI18n({
	legacy: false,
	locale: 'zh-CN',
	fallbackLocale: false,
	messageCompiler: createMessageCompiler(),
	missingWarn: import.meta.env.DEV,
	messages: buildLocaleMessages({
		'./app/zh-CN/index.json': { default: chineseApp },
		'./ui/zh-CN/index.json': { default: chineseUi },
	}),
})

// Existing preferences may contain a former upstream locale.
export async function setLocale(_requestedLocale: string): Promise<void> {
	i18n.global.locale.value = 'zh-CN'
	document.documentElement.lang = 'zh-CN'
}
export default i18n
