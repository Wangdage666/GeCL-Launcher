<script setup lang="ts">
import { Channel, invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useQueryClient } from '@tanstack/vue-query'
import { nextTick, onMounted, ref } from 'vue'
import { XIcon } from '@modrinth/assets'
import { instanceKeys } from '@/pages/instance/query-options'

type Game = {
	source: string
	versionFile: string
	name: string
	gameVersion: string
	loader: string
	loaderVersion: string | null
	mods: string[]
	shaderpacks: string[]
	resourcepacks: string[]
	worlds: string[]
}
const emit = defineEmits<{ close: [] }>()
const dialog = ref<HTMLDialogElement>()
const games = ref<Game[]>([])
const selected = ref(0)
const busy = ref(false)
const importing = ref(false)
const error = ref('')
const success = ref('')
const progress = ref<[number, number]>([0, 0])
const jobId = ref('')
const queryClient = useQueryClient()
const contentGroups = [
	['mods', '模组文件'],
	['shaderpacks', '光影文件'],
	['resourcepacks', '资源包文件'],
] as const
const loaderNames: Record<string, string> = {
	vanilla: '原版',
	fabric: 'Fabric',
	forge: 'Forge',
	neoforge: 'NeoForge',
	quilt: 'Quilt',
}
const message = (e: unknown) => (e as { message?: string })?.message ?? String(e)

async function choose() {
	let directory: string | string[] | null
	try {
		directory = await open({ directory: true, multiple: false, title: '选择已安装的游戏文件夹' })
	} catch (e) {
		error.value = message(e)
		return
	}
	if (typeof directory !== 'string') return
	busy.value = true
	error.value = ''
	success.value = ''
	games.value = []
	try {
		games.value = await invoke<Game[]>('plugin:gecl|preview_folder', { path: directory })
		selected.value = 0
	} catch (e) {
		error.value = message(e)
	} finally {
		busy.value = false
	}
}
async function importGame() {
	const game = games.value[selected.value]
	if (!game || busy.value) return
	busy.value = true
	importing.value = true
	error.value = ''
	success.value = ''
	progress.value = [0, 0]
	jobId.value = crypto.randomUUID()
	const channel = new Channel<[number, number]>()
	channel.onmessage = (value) => {
		progress.value = value
	}
	try {
		await invoke('plugin:gecl|import_folder', {
			jobId: jobId.value,
			source: game.source,
			versionFile: game.versionFile,
			onProgress: channel,
		})
		await queryClient.invalidateQueries({ queryKey: instanceKeys.all })
		success.value = `已导入 ${game.name}。首次使用时请补全运行环境。`
		games.value = []
	} catch (e) {
		error.value = message(e)
	} finally {
		busy.value = false
		importing.value = false
		jobId.value = ''
	}
}
async function cancel() {
	if (jobId.value) await invoke('plugin:gecl|cancel_import', { jobId: jobId.value })
}
function close() {
	if (!busy.value) {
		dialog.value?.close()
		emit('close')
	}
}
onMounted(async () => {
	await nextTick()
	dialog.value?.showModal()
})
</script>

<template>
	<dialog
		ref="dialog"
		class="folder-import"
		aria-labelledby="folder-import-title"
		@cancel.prevent="close"
	>
		<header>
			<h1 id="folder-import-title">从文件夹导入游戏</h1>
			<button :disabled="busy" aria-label="关闭" @click="close"><XIcon class="size-5" /></button>
		</header>
		<p>选择单个游戏版本文件夹，或包含 versions 的 .minecraft 目录。</p>
		<button :disabled="busy" @click="choose">
			{{ busy && !importing ? '正在识别…' : '选择文件夹' }}
		</button>
		<p v-if="error" role="alert" class="error">{{ error }}</p>
		<p v-if="success" role="status">{{ success }}</p>
		<div v-if="games.length" class="results">
			<label v-for="(game, index) in games" :key="game.versionFile" class="game-row">
				<input v-model="selected" type="radio" name="import-game" :value="index" :disabled="busy" />
				<span
					><strong>{{ game.name }}</strong
					><small
						>Minecraft {{ game.gameVersion }} · {{ loaderNames[game.loader] ?? game.loader }}
						{{ game.loaderVersion ?? '' }}</small
					></span
				>
			</label>
			<dl>
				<dt>模组</dt>
				<dd>{{ games[selected].mods.length }}</dd>
				<dt>光影包</dt>
				<dd>{{ games[selected].shaderpacks.length }}</dd>
				<dt>资源包</dt>
				<dd>{{ games[selected].resourcepacks.length }}</dd>
				<dt>存档</dt>
				<dd>{{ games[selected].worlds.length }}</dd>
			</dl>
			<details v-for="[key, label] in contentGroups" :key="key">
				<summary>{{ label }}</summary>
				<ul>
					<li v-for="name in games[selected][key]" :key="name">{{ name }}</li>
				</ul>
			</details>
			<p class="help">
				导入前请退出游戏。已在 profiles
				中的版本直接登记，其他目录复制到游戏数据目录；不移动源文件。缺少的运行文件会在安装时补全。
			</p>
		</div>
		<div v-if="importing" role="status">
			<p>
				正在复制并登记游戏… {{ Math.floor(progress[0] / 1048576) }} /
				{{ Math.floor(progress[1] / 1048576) }} MB
			</p>
			<progress :value="progress[0]" :max="progress[1] || 1" />
		</div>
		<footer>
			<button v-if="importing" @click="cancel">取消导入</button
			><button v-else @click="close">{{ success ? '完成' : '取消' }}</button
			><button v-if="games.length" class="primary" :disabled="busy" @click="importGame">
				导入所选版本
			</button>
		</footer>
	</dialog>
</template>

<style scoped>
.folder-import {
	box-sizing: border-box;
	width: min(760px, calc(100vw - 48px));
	max-height: 85vh;
	padding: 26px;
	border: 0;
	border-radius: 16px;
	background: var(--color-surface-1, #fff);
	color: var(--color-contrast, #202832);
	overflow: auto;
}
.folder-import::backdrop {
	background: #11182766;
}
header {
	display: flex;
	align-items: center;
	justify-content: space-between;
}
h1 {
	font-size: 22px;
	margin: 0;
}
header button {
	border: 0;
	background: transparent;
	font-size: 26px;
}
p {
	line-height: 1.5;
}
button {
	cursor: pointer;
	font: inherit;
	background: #fff;
	color: inherit;
	border: 1px solid #cfd6dd;
	border-radius: 8px;
	padding: 8px 14px;
}
.primary {
	background: #1479ca;
	border-color: #1479ca;
	color: white;
}
button:disabled {
	opacity: 0.5;
	cursor: default;
}
button:focus-visible,
input:focus-visible,
summary:focus-visible {
	outline: 2px solid #1479ca;
	outline-offset: 3px;
}
.results {
	margin-top: 20px;
}
.game-row {
	display: flex;
	align-items: center;
	gap: 10px;
	padding: 10px 0;
	border-bottom: 1px solid #e3e7eb;
}
.game-row span {
	min-width: 0;
	overflow-wrap: anywhere;
}
small {
	display: block;
	margin-top: 5px;
	color: #596370;
}
.game-row input[type='radio'] {
	appearance: auto;
	width: 16px;
	height: 16px;
	accent-color: #1479ca;
	flex: none;
}
dl {
	display: grid;
	grid-template-columns: auto 1fr auto 1fr;
	gap: 10px;
	margin: 18px 0;
}
dd {
	margin: 0;
	font-weight: 600;
}
details {
	margin: 10px 0;
}
summary {
	cursor: pointer;
}
ul {
	font-size: 13px;
	max-height: 130px;
	overflow: auto;
	overflow-wrap: anywhere;
}
.help {
	font-size: 13px;
	color: #596370;
}
.error {
	color: #b91c1c;
}
footer {
	display: flex;
	justify-content: flex-end;
	gap: 10px;
	margin-top: 24px;
}
progress {
	width: 100%;
	accent-color: #1479ca;
}
</style>
