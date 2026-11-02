<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import { nextTick, onMounted, ref } from 'vue'
import { XIcon } from '@modrinth/assets'
import App from '@/App.vue'

const ready = ref(false)
const needsDirectory = ref(false)
const firstRun = ref(false)
const directory = ref('')
const busy = ref(false)
const error = ref('')
const dialog = ref<HTMLDialogElement>()
const message = (error: unknown) => (error as { message?: string })?.message ?? String(error)

async function load() {
	error.value = ''
	try {
		const status = await invoke<{
			needsDirectory: boolean
			needsAccount: boolean
			dataDirectory: string | null
		}>('plugin:gecl|setup_status')
		firstRun.value = status.needsAccount
		needsDirectory.value = status.needsDirectory
		if (status.needsDirectory) directory.value = status.dataDirectory ?? ''
		ready.value = !status.needsDirectory
		if (status.needsDirectory) {
			await nextTick()
			dialog.value?.showModal()
			await invoke('show_window')
		}
	} catch (e) {
		error.value = message(e)
		await nextTick()
		dialog.value?.showModal()
		await getCurrentWindow().show()
	}
}
async function choose() {
	try {
		const selected = await open({ directory: true, multiple: false, title: '选择游戏数据目录' })
		if (typeof selected === 'string') directory.value = selected
	} catch (e) {
		error.value = message(e)
	}
}
async function next() {
	if (!directory.value || busy.value) return
	busy.value = true
	error.value = ''
	try {
		await invoke('plugin:gecl|choose_directory', { path: directory.value })
		ready.value = true
		firstRun.value = true
	} catch (e) {
		error.value = message(e)
	} finally {
		busy.value = false
	}
}
onMounted(load)
</script>

<template>
	<App v-if="ready" :first-run="firstRun" />
	<main v-else class="gecl-startup">
		<header data-tauri-drag-region><span>gecl</span></header>
		<dialog
			v-if="needsDirectory || error"
			ref="dialog"
			aria-labelledby="setup-directory-title"
			@cancel.prevent
		>
			<div class="dialog-heading">
				<h1 id="setup-directory-title">
					{{ needsDirectory ? '选择游戏数据目录' : '无法读取设置' }}
				</h1>
				<button class="close" aria-label="关闭窗口" @click="getCurrentWindow().close()">
					<XIcon />
				</button>
			</div>
			<template v-if="needsDirectory">
				<p>游戏、模组、存档和下载文件将保存在这里。</p>
				<label for="gecl-data-directory">数据目录</label>
				<div class="directory-row">
					<input
						id="gecl-data-directory"
						v-model="directory"
						readonly
						placeholder="尚未选择"
					/><button :disabled="busy" @click="choose">选择文件夹</button>
				</div>
				<p v-if="error" role="alert" class="error">{{ error }}</p>
				<footer>
					<button :disabled="busy" @click="getCurrentWindow().close()">退出</button>
					<button class="primary" :disabled="!directory || busy" @click="next">
						{{ busy ? '正在设置…' : '下一步' }}
					</button>
				</footer>
			</template>
			<template v-else
				><p role="alert" class="error">{{ error }}</p>
				<footer><button class="primary" @click="load">重试</button></footer></template
			>
		</dialog>
		<p v-else class="loading" role="status">正在读取设置…</p>
	</main>
</template>

<style scoped>
.gecl-startup {
	min-height: 100vh;
	background: #f8f9fa;
	color: #202832;
	font-family: inherit;
}
header {
	height: 56px;
	display: flex;
	align-items: center;
	justify-content: space-between;
	padding: 0 20px;
	border-bottom: 1px solid #e3e7eb;
	font-size: 24px;
	font-weight: 700;
}
dialog {
	width: min(560px, calc(100vw - 48px));
	max-height: calc(100vh - 48px);
	box-sizing: border-box;
	padding: 24px;
	border: 0;
	border-radius: 14px;
	background: #fff;
	color: inherit;
	box-shadow: 0 12px 40px #16263726;
}
dialog::backdrop {
	background: #1118274d;
}
.dialog-heading {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 16px;
}
.close {
	border: 0;
	background: transparent;
	padding: 6px;
	display: flex;
}
.close svg {
	width: 20px;
	height: 20px;
}
.loading {
	margin: 32px;
	font-size: 14px;
	color: #596370;
}
h1 {
	font-size: 20px;
	margin: 0;
}
p {
	line-height: 1.6;
	margin: 12px 0 22px;
}
label {
	display: block;
	font-weight: 600;
	margin-bottom: 8px;
}
.directory-row {
	display: flex;
	gap: 10px;
}
input {
	flex: 1;
	min-width: 0;
	background: #f8f9fa;
	color: inherit;
	border: 1px solid #cfd6dd;
	border-radius: 8px;
	padding: 10px;
}
button {
	cursor: pointer;
	font: inherit;
	border: 1px solid #cfd6dd;
	border-radius: 8px;
	padding: 9px 14px;
	background: #fff;
	color: inherit;
	transition:
		background-color 150ms ease,
		border-color 150ms ease;
}
button:hover:not(:disabled) {
	background: #edf4fa;
	border-color: #1479ca;
}
button:active:not(:disabled) {
	background: #dfeef9;
}
button:disabled {
	opacity: 0.5;
	cursor: default;
}
button:focus-visible,
input:focus-visible {
	outline: 2px solid #1479ca;
	outline-offset: 3px;
}
.primary {
	background: #1479ca;
	color: #fff;
	border-color: #1479ca;
}
.primary:hover:not(:disabled) {
	background: #116bb3;
}
.primary:active:not(:disabled) {
	background: #0f5e9d;
}
footer {
	display: flex;
	justify-content: flex-end;
	gap: 10px;
	margin-top: 24px;
}
.error {
	color: #b91c1c;
}
@media (prefers-reduced-motion: reduce) {
	button {
		transition: none;
	}
}
</style>
