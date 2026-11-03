<script setup lang="ts">
import { Button, NewModal, ProgressBar } from '@modrinth/ui'
import { Channel, invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import { get_all as runningProcesses } from '@/helpers/process'

interface UpdateInfo {
	version: string
	current: string
	notes: string
}

const modal = ref<InstanceType<typeof NewModal>>()
const info = ref<UpdateInfo | null>(null)
const phase = ref<'ask' | 'downloading' | 'installing'>('ask')
const progress = ref({ done: 0, total: 1 })
const error = ref('')
const message = (value: unknown) =>
	value instanceof Error ? value.message : String((value as { message?: string })?.message ?? value)

onMounted(async () => {
	try {
		info.value = await invoke<UpdateInfo | null>('plugin:gecl|update_check')
		if (info.value) modal.value?.show()
	} catch (value) {
		console.warn('GeCL update check failed:', value)
	}
})

async function update() {
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
		phase.value = 'ask'
		error.value = message(value)
	}
}

async function dismiss() {
	if (phase.value === 'downloading') await invoke('plugin:gecl|update_cancel').catch(() => {})
	else modal.value?.hide()
}

const megabytes = (bytes: number) => (bytes / 1048576).toFixed(1)
</script>
<template>
	<NewModal
		ref="modal"
		header="发现新版本"
		width="32rem"
		:closable="phase === 'ask'"
		:close-on-esc="phase === 'ask'"
		:close-on-click-outside="false"
		:before-hide="() => phase === 'ask'"
	>
		<div v-if="info" class="flex flex-col gap-3">
			<p class="m-0 text-contrast">
				gecl {{ info.version }} 可用，当前 {{ info.current }}
			</p>
			<div
				v-if="info.notes.trim() && phase === 'ask'"
				class="max-h-48 overflow-y-auto whitespace-pre-wrap break-words rounded-lg bg-surface-2 p-3 text-sm"
			>
				{{ info.notes.trim() }}
			</div>
			<ProgressBar
				v-if="phase !== 'ask'"
				:progress="progress.done"
				:max="Math.max(progress.total, 1)"
				full-width
				:label="
					phase === 'installing'
						? '正在安装，完成后自动重启'
						: `正在下载 ${megabytes(progress.done)} / ${megabytes(progress.total)} MB`
				"
			/>
			<p v-if="error" role="alert" class="m-0 break-words text-red">{{ error }}</p>
		</div>
		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<Button type="outlined" :disabled="phase === 'installing'" @click="dismiss">{{
					phase === 'downloading' ? '取消' : '以后'
				}}</Button>
				<Button type="colored" color="brand" :disabled="phase !== 'ask'" @click="update">{{
					error && phase === 'ask' ? '重试' : '更新'
				}}</Button>
			</div>
		</template>
	</NewModal>
</template>
