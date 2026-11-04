<script setup lang="ts">
import { Button, NewModal, ProgressBar } from '@modrinth/ui'
import { onMounted, ref } from 'vue'

import { downloadProgressLabel, INSTALLING_LABEL, useAppUpdate } from './use-app-update'

const modal = ref<InstanceType<typeof NewModal>>()
const { info, phase, progress, error, cancelling, autoCheck, check, install, cancel } =
	useAppUpdate()

onMounted(async () => {
	if (autoCheck.value && (await check())) modal.value?.show()
})

async function dismiss() {
	if (phase.value === 'downloading') await cancel()
	else modal.value?.hide()
}
</script>
<template>
	<NewModal
		ref="modal"
		header="发现新版本"
		width="32rem"
		:closable="phase === 'idle'"
		:close-on-esc="phase === 'idle'"
		:close-on-click-outside="false"
		:before-hide="() => phase === 'idle'"
	>
		<div v-if="info" class="flex flex-col gap-3">
			<p class="m-0 text-contrast">gecl {{ info.version }} 可用，当前 {{ info.current }}</p>
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
		</div>
		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<Button type="outlined" :disabled="phase === 'installing' || cancelling" @click="dismiss">{{
					phase === 'downloading' ? (cancelling ? '正在取消…' : '取消') : '以后'
				}}</Button>
				<Button type="colored" color="brand" :disabled="phase !== 'idle'" @click="install">{{
					error && phase === 'idle' ? '重试' : '更新'
				}}</Button>
			</div>
		</template>
	</NewModal>
</template>
