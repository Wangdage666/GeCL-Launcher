import { Channel, invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

import { get_all as runningProcesses } from '@/helpers/process'

export interface AppUpdateInfo {
	version: string
	current: string
	notes: string
	pageUrl: string
}

const AUTO_CHECK_KEY = 'gecl-update-auto-check'
const LAST_CHECK_KEY = 'gecl-update-last-check'

function readStored(key: string): string | null {
	try {
		return localStorage.getItem(key)
	} catch {
		return null
	}
}
function writeStored(key: string, value: string) {
	try {
		localStorage.setItem(key, value)
	} catch {
		/* Optional preference. */
	}
}
const message = (value: unknown) =>
	value instanceof Error ? value.message : String((value as { message?: string })?.message ?? value)

const storedLastCheck = Number(readStored(LAST_CHECK_KEY))

const info = ref<AppUpdateInfo | null>(null)
const phase = ref<'idle' | 'downloading' | 'installing'>('idle')
const progress = ref({ done: 0, total: 1 })
const error = ref('')
const checking = ref(false)
const checkedThisSession = ref(false)
const checkError = ref('')
const cancelling = ref(false)
const lastChecked = ref<number | null>(storedLastCheck > 0 ? storedLastCheck : null)
const autoCheck = ref(readStored(AUTO_CHECK_KEY) !== 'false')

let pendingCheck: Promise<AppUpdateInfo | null> | null = null

/** One check at a time, shared by the startup prompt and the settings page. */
function check(): Promise<AppUpdateInfo | null> {
	if (phase.value !== 'idle') return Promise.resolve(info.value)
	pendingCheck ??= (async () => {
		checking.value = true
		checkError.value = ''
		try {
			info.value = await invoke<AppUpdateInfo | null>('plugin:gecl|update_check')
			checkedThisSession.value = true
			lastChecked.value = Date.now()
			writeStored(LAST_CHECK_KEY, String(lastChecked.value))
			error.value = ''
		} catch (value) {
			checkError.value = message(value)
		} finally {
			checking.value = false
			pendingCheck = null
		}
		return info.value
	})()
	return pendingCheck
}

/** Downloads the checked release, then the backend starts the installer and closes the app. */
async function install() {
	if (phase.value !== 'idle' || !info.value) return
	error.value = ''
	try {
		if (((await runningProcesses()) as unknown[]).length) {
			error.value = '请先关闭正在运行的游戏'
			return
		}
		phase.value = 'downloading'
		progress.value = { done: 0, total: 1 }
		const onProgress = new Channel<{ done: number; total: number }>()
		onProgress.onmessage = (value) => {
			progress.value = value
		}
		await invoke('plugin:gecl|update_download', { onProgress })
		phase.value = 'installing'
		await invoke('plugin:gecl|update_install')
	} catch (value) {
		phase.value = 'idle'
		if (!cancelling.value) error.value = message(value)
	} finally {
		cancelling.value = false
	}
}

async function cancel() {
	if (phase.value !== 'downloading' || cancelling.value) return
	cancelling.value = true
	await invoke('plugin:gecl|update_cancel').catch(() => {
		cancelling.value = false
	})
}

function setAutoCheck(value: boolean) {
	autoCheck.value = value
	writeStored(AUTO_CHECK_KEY, String(value))
}

export function useAppUpdate() {
	return {
		info,
		phase,
		progress,
		error,
		checking,
		checkedThisSession,
		checkError,
		cancelling,
		lastChecked,
		autoCheck,
		check,
		install,
		cancel,
		setAutoCheck,
	}
}

export const downloadProgressLabel = (done: number, total: number) =>
	`正在下载 ${(done / 1048576).toFixed(1)} / ${(total / 1048576).toFixed(1)} MB`
export const INSTALLING_LABEL = '正在安装，完成后自动重启'
