<script setup lang="ts">
import CurseForgeDownloads from '@/features/curseforge/DownloadTasks.vue'
import { downloadTasks } from '@/features/curseforge/tasks'
import { BoxIcon } from '@modrinth/assets'
import { Button } from '@modrinth/ui'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import { useAppEvent } from '@/composables/use-app-event'
import DownloadManagerJob from '@/components/ui/download-manager/download-manager-job.vue'
import { useDownloadManager } from '@/components/ui/download-manager/use-download-manager'
import { instanceKeys, instanceListQueryOptions } from '@/pages/instance/query-options'
const instances = useQuery(instanceListQueryOptions())
const queryClient = useQueryClient()
const recent = computed(() =>
	(instances.data.value ?? [])
		.filter((instance) => instance.last_played && new Date(instance.last_played).getTime() > 0)
		.sort((a, b) => new Date(b.last_played!).getTime() - new Date(a.last_played!).getTime())
		.slice(0, 3),
)
useAppEvent('process', (event) => {
	if (event.event === 'launched' || event.event === 'finished')
		void queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
})
const { activeJobs, attentionJobs, rate, retry, cancel, togglePause, dismiss, copyDetails } =
	useDownloadManager()
const jobs = computed(() => [...attentionJobs.value, ...activeJobs.value])
</script>
<template>
	<section
		v-if="recent.length || instances.isError.value"
		class="p-4 border-0 border-b border-solid border-[--brand-gradient-border]"
	>
		<h2 class="m-0 mb-2 text-base font-semibold text-contrast">最近游玩</h2>
		<template v-if="instances.isError.value"
			><p class="m-0 text-sm text-red">读取游戏列表失败</p>
			<Button size="sm" @click="instances.refetch()">重试</Button></template
		>
		<RouterLink
			v-for="instance in recent"
			:key="instance.id"
			:to="`/instance/${encodeURIComponent(instance.id)}`"
			class="flex gap-2 items-center px-1 py-2 rounded-lg no-underline text-inherit hover:bg-button-bg"
		>
			<BoxIcon class="size-6 shrink-0 text-blue" />
			<div class="min-w-0">
				<div class="truncate text-sm font-medium text-contrast">{{ instance.name }}</div>
				<div class="text-xs text-secondary">
					{{ instance.game_version }} ·
					{{ instance.loader === 'vanilla' ? '原版' : instance.loader }}
				</div>
			</div>
		</RouterLink>
	</section>
	<section v-if="jobs.length || downloadTasks.length" class="p-3">
		<div class="flex justify-between items-center px-1 mb-2">
			<h2 class="m-0 text-base font-semibold text-contrast">下载任务</h2>
			<span v-if="jobs.length" class="text-xs text-secondary tabular-nums">{{ rate }}</span>
		</div>
		<CurseForgeDownloads />
		<DownloadManagerJob
			v-for="job in jobs"
			:key="job.id"
			:job="job"
			@retry="retry"
			@cancel="cancel"
			@toggle-pause="togglePause"
			@dismiss="dismiss"
			@copy-details="copyDetails"
		/>
	</section>
</template>
