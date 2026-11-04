<script setup lang="ts">
import { ExternalIcon, RefreshCwIcon } from '@modrinth/assets'
import { Button, ProgressBar, Toggle } from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, onMounted, ref } from 'vue'

import {
	downloadProgressLabel,
	INSTALLING_LABEL,
	useAppUpdate,
} from '@/features/gecl/use-app-update'

const {
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
} = useAppUpdate()

const version = ref(await getVersion())
const busy = computed(() => checking.value || phase.value !== 'idle')
const autoCheckEnabled = computed({
	get: () => autoCheck.value,
	set: setAutoCheck,
})
const lastCheckedLabel = computed(() =>
	lastChecked.value
		? new Date(lastChecked.value).toLocaleString('zh-CN', {
				month: 'numeric',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit',
			})
		: '',
)
const status = computed(() => {
	if (checking.value) return '正在检查…'
	if (info.value) return `发现新版本 ${info.value.version}`
	if (checkedThisSession.value) return '已是最新版本'
	return ''
})

onMounted(() => {
	if (!checkedThisSession.value) void check()
})
</script>
<template>
	<div class="flex flex-col gap-6">
		<div class="flex items-center justify-between gap-4">
			<div class="min-w-0">
				<h3 class="m-0 text-lg font-semibold text-contrast">gecl {{ version }}</h3>
				<p class="m-0 mt-1" aria-live="polite">
					<span v-if="status">{{ status }}</span>
					<span v-if="status && lastCheckedLabel && !checking" class="text-secondary">
						· 上次检查 {{ lastCheckedLabel }}</span
					>
					<span v-else-if="!status && lastCheckedLabel" class="text-secondary"
						>上次检查 {{ lastCheckedLabel }}</span
					>
				</p>
			</div>
			<Button :disabled="busy" class="shrink-0" @click="check">
				<RefreshCwIcon :class="{ 'animate-spin': checking }" />
				检查更新
			</Button>
		</div>

		<p v-if="checkError" role="alert" class="m-0 break-words text-red">
			检查失败：{{ checkError }}
		</p>

		<section
			v-if="info"
			aria-label="新版本"
			class="flex flex-col gap-3 border-0 border-t border-solid border-surface-5 pt-5"
		>
			<div class="flex items-center justify-between gap-4">
				<h3 class="m-0 text-lg font-semibold text-contrast">gecl {{ info.version }}</h3>
				<div class="flex shrink-0 items-center gap-2">
					<Button v-if="phase === 'idle'" @click="openUrl(info.pageUrl)"
						>发布页 <ExternalIcon
					/></Button>
					<Button
						v-if="phase === 'downloading'"
						type="outlined"
						:disabled="cancelling"
						@click="cancel"
						>{{ cancelling ? '正在取消…' : '取消' }}</Button
					>
					<Button
						type="colored"
						color="brand"
						:disabled="phase !== 'idle'"
						@click="install"
						>{{ error && phase === 'idle' ? '重试' : '更新' }}</Button
					>
				</div>
			</div>
			<div
				v-if="info.notes.trim() && phase === 'idle'"
				class="max-h-48 overflow-y-auto whitespace-pre-wrap break-words text-sm"
			>
				{{ info.notes.trim() }}
			</div>
			<ProgressBar
				v-if="phase !== 'idle'"
				:progress="progress.done"
				:max="Math.max(progress.total, 1)"
				full-width
				:label="
					phase === 'installing' ? INSTALLING_LABEL : downloadProgressLabel(progress.done, progress.total)
				"
			/>
			<p v-if="error" role="alert" class="m-0 break-words text-red">{{ error }}</p>
		</section>

		<div
			class="flex items-center justify-between gap-4 border-0 border-t border-solid border-surface-5 pt-5"
		>
			<h3 id="gecl-auto-update-check" class="m-0 text-lg font-semibold text-contrast">
				启动时检查更新
			</h3>
			<Toggle
				id="gecl-auto-update-check-toggle"
				v-model="autoCheckEnabled"
				aria-labelledby="gecl-auto-update-check"
			/>
		</div>
	</div>
</template>
