import { invoke } from '@tauri-apps/api/core'
import { computed, ref, shallowRef } from 'vue'
import type { ColorTheme } from '@/composables/use-theme'

import { DEFAULT_VISUAL_PREFERENCES, type VisualPreferences } from './palette'

export type Background = { id: string; path: string }
export type VisualAppearance = { preferences: VisualPreferences; background: Background | null }

const saved = shallowRef<VisualAppearance>({
	preferences: { ...DEFAULT_VISUAL_PREFERENCES },
	background: null,
})
const preview = shallowRef<VisualAppearance | null>(null)
const loaded = ref(false)
let loading: Promise<void> | null = null
let backgroundOperations = Promise.resolve()

function queueBackgroundOperation<T>(operation: () => Promise<T>): Promise<T> {
	const result = backgroundOperations.then(operation)
	backgroundOperations = result.then(
		() => {},
		() => {},
	)
	return result
}

async function load(): Promise<void> {
	if (loaded.value) return
	if (!loading) {
		loading = invoke<VisualAppearance>('plugin:gecl|appearance_get')
			.then((value) => {
				saved.value = value
				loaded.value = true
			})
			.finally(() => {
				loading = null
			})
	}
	return loading
}

async function persist(
	preferences: VisualPreferences,
	window: {
		theme: ColorTheme
		advancedRendering: boolean
		nativeDecorations: boolean
	},
): Promise<void> {
	saved.value = await invoke<VisualAppearance>('plugin:gecl|appearance_set', {
		preferences,
		window,
	})
}

export const visualAppearance = {
	saved,
	preview,
	loaded,
	active: computed(() => preview.value ?? saved.value),
	load,
	persist,
}

export function discardBackground(id: string): Promise<void> {
	return queueBackgroundOperation(() => invoke('plugin:gecl|appearance_discard_background', { id }))
}

export function importBackground(source: string): Promise<Background> {
	return queueBackgroundOperation(() =>
		invoke('plugin:gecl|appearance_import_background', { source }),
	)
}
