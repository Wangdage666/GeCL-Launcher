<script setup lang="ts">
import { Button, Input, NewModal, ProgressBar } from '@modrinth/ui'
import { Channel, invoke } from '@tauri-apps/api/core'
import { join } from '@tauri-apps/api/path'
import { open } from '@tauri-apps/plugin-dialog'
import { ref } from 'vue'
import type { SingleplayerWorld } from '@/helpers/worlds'

const props = defineProps<{ instanceId: string; worlds: SingleplayerWorld[] }>()
const emit = defineEmits<{ restored: [] }>()
const modal = ref<InstanceType<typeof NewModal>>()
const world = ref('')
const restore = ref(false)
const directory = ref('')
const archive = ref('')
const busy = ref(false)
const cancelling = ref(false)
const error = ref('')
const completed = ref('')
const recovery = ref('')
const progress = ref({ done: 0, total: 1 })
let jobId = ''
async function show(selected?: SingleplayerWorld, restoring = false) {
	world.value = selected?.path ?? props.worlds[0]?.path ?? ''
	restore.value = restoring
	archive.value = ''
	error.value = ''
	completed.value = ''
	recovery.value = ''
	directory.value = (await invoke<string | null>('plugin:gecl|world_backup_directory')) ?? ''
	modal.value?.show()
}
async function choose() {
	const selected = await open(
		restore.value
			? {
					multiple: false,
					filters: [{ name: '世界备份', extensions: ['zip'] }],
					defaultPath: directory.value || undefined,
				}
			: { multiple: false, directory: true, defaultPath: directory.value || undefined },
	)
	if (typeof selected !== 'string') return
	if (restore.value) archive.value = selected
	else directory.value = selected
}
async function run() {
	error.value = ''
	completed.value = ''
	recovery.value = ''
	cancelling.value = false
	if (!world.value || !(restore.value ? archive.value : directory.value)) return
	busy.value = true
	jobId = crypto.randomUUID()
	progress.value = { done: 0, total: 1 }
	try {
		const timestamp = new Date()
			.toISOString()
			.replaceAll(/[:.]/g, '-')
			.replace('T', '_')
			.replace('Z', '')
		const path = restore.value
			? archive.value
			: await join(directory.value, `${world.value}_${timestamp}.zip`)
		const onProgress = new Channel<{ done: number; total: number }>()
		onProgress.onmessage = (value) => {
			progress.value = value
		}
		const result = await invoke<{ path: string; recovery: string | null }>(
			'plugin:gecl|world_backup',
			{
				jobId,
				instanceId: props.instanceId,
				world: world.value,
				archive: path,
				restore: restore.value,
				onProgress,
			},
		)
		completed.value = restore.value ? '恢复完成' : `备份完成：${result.path}`
		recovery.value = result.recovery ?? ''
		if (restore.value) emit('restored')
	} catch (value) {
		error.value =
			value instanceof Error
				? value.message
				: String((value as { message?: string })?.message ?? value)
	} finally {
		busy.value = false
		jobId = ''
	}
}
async function cancel() {
	if (busy.value && jobId) {
		cancelling.value = true
		await invoke('plugin:gecl|cancel_world_backup', { jobId })
	} else modal.value?.hide()
}
defineExpose({ show })
</script>
<template>
	<NewModal
		ref="modal"
		:header="restore ? '恢复存档' : '备份存档'"
		width="36rem"
		:closable="!busy"
		:close-on-esc="!busy"
		:close-on-click-outside="!busy"
		:before-hide="() => !busy"
	>
		<div class="flex flex-col gap-4">
			<label for="backup-world" class="font-semibold text-contrast">世界</label>
			<select
				id="backup-world"
				style="appearance: auto; background-image: none"
				v-model="world"
				:disabled="busy"
				class="rounded-lg border border-solid border-surface-5 bg-surface-3 p-2 text-contrast"
			>
				<option v-for="item in worlds" :key="item.path" :value="item.path">{{ item.name }}</option>
			</select>
			<div class="flex min-w-0 items-center gap-2">
				<Input
					:model-value="restore ? archive : directory"
					readonly
					:placeholder="restore ? '选择备份文件' : '选择备份位置'"
					wrapper-class="min-w-0 flex-1"
				/><Button type="outlined" :disabled="busy" @click="choose"
					>选择{{ restore ? '文件' : '文件夹' }}</Button
				>
			</div>
			<p v-if="restore" class="m-0 text-orange">将覆盖所选世界，当前存档会先另存一份。</p>
			<ProgressBar
				v-if="busy"
				:progress="progress.done"
				:max="Math.max(progress.total, 1)"
				full-width
				show-progress
				:label="restore ? '正在恢复' : '正在备份'"
			/>
			<p v-if="error" role="alert" class="m-0 break-words text-red">{{ error }}</p>
			<p v-if="completed" role="status" class="m-0 break-all text-green">{{ completed }}</p>
			<p v-if="recovery" class="m-0 break-all">恢复前副本：{{ recovery }}</p>
		</div>
		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<Button type="outlined" :disabled="cancelling" @click="cancel">{{
					busy ? (cancelling ? '正在取消…' : '取消') : '关闭'
				}}</Button>
				<Button
					type="colored"
					color="brand"
					:disabled="busy || !world || !(restore ? archive : directory)"
					@click="run"
					>{{ restore ? '恢复并覆盖' : '备份' }}</Button
				>
			</div>
		</template>
	</NewModal>
</template>
