<script setup lang="ts">
import { Button, ProgressBar } from '@modrinth/ui'
import { downloadTasks, dismissDownload, startDownload, stopDownload } from './tasks'
function retry(task: (typeof downloadTasks.value)[number]) {
	dismissDownload(task)
	startDownload(task)
}
</script>
<template>
	<div
		v-for="task in downloadTasks"
		:key="task.id"
		class="flex flex-col gap-2 p-2 mb-2 rounded-lg bg-surface-2"
		role="status"
	>
		<div class="flex items-center gap-2">
			<img v-if="task.icon" :src="task.icon" alt="" class="size-9 rounded-lg" /><RouterLink
				:to="`/curseforge/${task.projectId}?i=${encodeURIComponent(task.instanceId)}`"
				class="min-w-0 truncate font-semibold text-contrast"
				>{{ task.title }}</RouterLink
			><span class="text-xs text-secondary">CurseForge</span>
		</div>
		<div class="text-sm">
			{{
				task.status === 'canceling'
					? '正在取消并还原…'
					: task.status === 'canceled'
						? '已取消'
						: task.status === 'failed'
							? '安装失败'
							: task.progress.phase
			}}
		</div>
		<template v-if="task.status === 'running' || task.status === 'canceling'">
			<ProgressBar
				:progress="task.progress.total ? task.progress.completed / task.progress.total : 0"
				:waiting="!task.progress.total"
			/>
			<span class="truncate text-xs text-secondary" :title="task.progress.name">{{
				task.progress.name
			}}</span>
			<Button size="sm" :disabled="task.status === 'canceling'" @click="stopDownload(task)"
				>取消</Button
			>
		</template>
		<p v-if="task.error" class="m-0 break-words text-xs text-red">{{ task.error }}</p>
		<div v-if="!['running', 'canceling'].includes(task.status)" class="flex gap-2">
			<Button v-if="task.status === 'failed'" size="sm" @click="retry(task)">重试</Button
			><Button size="sm" @click="dismissDownload(task)">关闭</Button>
		</div>
	</div>
</template>
