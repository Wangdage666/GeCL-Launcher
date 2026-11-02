import { invoke } from '@tauri-apps/api/core'
export interface DownloadSourceSettings {
	policy: 'official_only' | 'official_preferred' | 'mirror_preferred' | 'mirror_only'
	slow_kib_per_second: number
	observation_seconds: number
}
export const getDownloadSources = () => invoke<DownloadSourceSettings>('plugin:settings|download_sources_get')
export const setDownloadSources = (settings: DownloadSourceSettings) => invoke<void>('plugin:settings|download_sources_set', { settings })
