<script setup lang="ts">
import { BoxIcon, XIcon } from '@modrinth/assets'
import { Button, IconButton, ProgressBar } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { RouterLink } from 'vue-router'

type Stage =
	| 'preparing'
	| 'checking_java'
	| 'checking_files'
	| 'starting_java'
	| 'loading_mods'
	| 'loading_resources'
	| 'window_ready'
type Launch = {
	id: string
	instance_id: string
	name: string
	stage: Stage
	percent: number
	started_at: number
	finished_at: number | null
	failed: boolean
}
const stages: Record<Stage, string> = {
	preparing: '准备启动',
	checking_java: '检查 Java',
	checking_files: '检查游戏文件',
	starting_java: '启动 Java',
	loading_mods: '加载模组',
	loading_resources: '加载游戏资源',
	window_ready: '游戏窗口已出现',
}
const launches = ref<Launch[]>([])
const now = ref(Date.now())
const error = ref(false)
const errorHidden = ref(false)
let stopped = false
let timer: ReturnType<typeof setTimeout> | undefined
const visible = computed(() => launches.value.length > 0 || (error.value && !errorHidden.value))
async function refresh() {
	try {
		const data = await invoke<Launch[]>('plugin:gecl|launch_progress')
		if (!stopped) {
			launches.value = data
			error.value = false
			errorHidden.value = false
		}
	} catch {
		if (!stopped) error.value = true
	}
	if (!stopped) {
		now.value = Date.now()
		timer = setTimeout(refresh, 700)
	}
}
function elapsed(launch: Launch) {
	const seconds = Math.max(
		0,
		Math.floor(((launch.finished_at ?? now.value) - launch.started_at) / 1000),
	)
	return seconds < 60 ? `${seconds} 秒` : `${Math.floor(seconds / 60)} 分 ${seconds % 60} 秒`
}
async function dismiss(launch: Launch) {
	try {
		await invoke('plugin:gecl|dismiss_launch', { id: launch.id })
		launches.value = launches.value.filter((item) => item.id !== launch.id)
	} catch {
		error.value = true
		errorHidden.value = false
	}
}
onMounted(refresh)
onUnmounted(() => {
	stopped = true
	clearTimeout(timer)
})
</script>

<template>
	<Transition name="launch-panel" appear>
		<aside
			v-if="visible"
			aria-label="游戏启动进度"
			class="launch-progress fixed bottom-4 right-4 z-40 flex w-[420px] max-w-[calc(100vw-2rem)] flex-col rounded-xl bg-bg-raised p-4 shadow-lg"
		>
			<div
				v-for="launch in launches"
				:key="launch.id"
				class="flex flex-col gap-2 py-1"
				:aria-busy="!launch.finished_at"
			>
				<div class="flex items-center gap-2">
					<BoxIcon class="size-6 shrink-0 text-blue" aria-hidden="true" />
					<strong class="min-w-0 flex-1 truncate text-sm text-contrast" :title="launch.name">{{
						launch.name
					}}</strong>
					<IconButton
						v-if="launch.finished_at"
						label="关闭启动提示"
						size="sm"
						type="quiet"
						@click="dismiss(launch)"
						><XIcon
					/></IconButton>
				</div>
				<div
					class="flex items-center justify-between gap-2 text-sm"
					role="status"
					aria-live="polite"
				>
					<span :class="launch.failed ? 'text-red' : 'text-primary'">{{
						launch.failed ? '启动未完成，请查看日志' : stages[launch.stage]
					}}</span>
					<span class="shrink-0 text-xs text-secondary tabular-nums">{{ launch.percent }}%</span>
				</div>
				<ProgressBar
					:progress="launch.percent"
					:max="100"
					:color="launch.failed ? 'red' : 'blue'"
					:label="`启动阶段：${launch.percent}%`"
					label-class="sr-only"
					full-width
				/>
				<div class="flex items-center justify-between gap-2">
					<span class="text-xs text-secondary tabular-nums"
						>{{ launch.finished_at ? '用时' : '已等待' }} {{ elapsed(launch) }}</span
					>
					<RouterLink
						:to="`/instance/${encodeURIComponent(launch.instance_id)}/logs`"
						class="text-xs text-link no-underline hover:underline"
						>查看日志</RouterLink
					>
				</div>
			</div>
			<div
				v-if="error && !errorHidden"
				class="flex items-center justify-between gap-2"
				role="status"
			>
				<span class="text-sm text-red">无法读取启动进度</span>
				<Button size="sm" @click="errorHidden = true">关闭提示</Button>
			</div>
		</aside>
	</Transition>
</template>

<style scoped>
.launch-panel-enter-active {
	transition: opacity 180ms ease-out;
}
.launch-panel-leave-active {
	transition: opacity 120ms ease-in;
}
.launch-panel-enter-from,
.launch-panel-leave-to {
	opacity: 0;
}
@media (prefers-reduced-motion: reduce) {
	.launch-panel-enter-active,
	.launch-panel-leave-active {
		transition: none;
	}
}
</style>
