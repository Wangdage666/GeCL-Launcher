<script setup lang="ts">
import {
	Button,
	ProjectCard,
	ProjectCardList,
	Pagination,
	LoadingIndicator,
	type BrowseInstallPreferences,
} from '@modrinth/ui'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { cachedMcmodEntry, displayModName, rememberMcmod, resolveMcmod } from '@/features/mcmod'
import { findChineseMods } from '@/features/mcmod/chinese-search'
import { mcmodProject, message, search, type Page, type Project } from './api'

const props = defineProps<{
	query: string
	preferences: BrowseInstallPreferences
	pageSize: number
	sort: string
	instanceId?: string
	layout: 'list' | 'grid'
	active: boolean
}>()
const route = useRoute()
const router = useRouter()
const result = ref<Page<Project> | null>(null)
const page = ref(1)
const loading = ref(false)
const error = ref('')
const note = ref('')
const translatedNames = ref<Record<number, string>>({})
async function translateName(project: Project) {
	const entry = await resolveMcmod(mcmodProject(project), controller?.signal)
	if (entry) translatedNames.value[project.id] = displayModName(project.name, entry)
}
let revision = 0
let controller: AbortController | undefined
let debounce: ReturnType<typeof setTimeout> | undefined
const pageCount = computed(() =>
	Math.max(
		1,
		Math.ceil(
			Math.min(10000, result.value?.pagination.totalCount ?? 0) / Math.min(props.pageSize, 50),
		),
	),
)
onBeforeUnmount(() => {
	clearTimeout(debounce)
	revision++
	controller?.abort()
})
async function load() {
	if (!props.active) return
	const current = ++revision
	controller?.abort()
	controller = new AbortController()
	const signal = controller.signal
	loading.value = true
	error.value = ''
	note.value = ''
	try {
		const versions = props.preferences.gameVersions ?? []
		const loaders = props.preferences.loaders ?? []
		if (versions.length > 1 || loaders.length > 1)
			throw new Error('CurseForge 请一次选择一个游戏版本和一种加载器。')
		const request = {
			query: props.query,
			gameVersion: versions[0],
			loader: loaders[0],
			index: (page.value - 1) * Math.min(props.pageSize, 50),
			pageSize: Math.min(props.pageSize, 50),
			sort: props.sort,
		}
		let response: Page<Project>
		if (/\p{Script=Han}/u.test(props.query)) {
			const entries = await findChineseMods(props.query, signal).catch((error) => {
				if (signal.aborted) throw error
				note.value = '中文名称对照暂不可用，已使用原词搜索。'
				return []
			})
			const slugs = [...new Set(entries.flatMap((entry) => entry.curseforgeSlugs ?? []))]
			if (slugs.length) {
				const responses = await Promise.allSettled(
					slugs.map((slug) => search({ ...request, query: '', slug, index: 0, pageSize: 50 })),
				)
				const successful = responses.filter(
					(r): r is PromiseFulfilledResult<Page<Project>> => r.status === 'fulfilled',
				)
				if (!successful.length) throw (responses[0] as PromiseRejectedResult).reason
				const matches = [
					...new Map(successful.flatMap((r) => r.value.data).map((p) => [p.id, p])).values(),
				]
				for (const project of matches) {
					const entry = entries.find((e) => e.curseforgeSlugs?.includes(project.slug))
					if (entry) rememberMcmod(mcmodProject(project), entry)
				}
				response = {
					data: matches.slice(request.index, request.index + request.pageSize),
					pagination: {
						index: request.index,
						pageSize: request.pageSize,
						resultCount: matches.length,
						totalCount: matches.length,
					},
				}
				if (successful.length < responses.length)
					note.value = '部分中文名称查询失败，已显示成功取得的结果。'
			} else {
				response = await search(request)
				note.value = '暂未找到这个中文名的 CurseForge 对照，可尝试英文名。'
			}
		} else response = await search(request)
		if (current === revision) result.value = response
	} catch (e) {
		if (current === revision) {
			error.value = message(e)
			result.value = null
		}
	} finally {
		if (current === revision) loading.value = false
	}
}
watch(
	() => [props.query, props.preferences, props.pageSize, props.sort, props.active],
	() => {
		clearTimeout(debounce)
		controller?.abort()
		revision++
		page.value = 1
		loading.value = props.active
		if (props.active) debounce = setTimeout(() => void load(), 350)
	},
	{ deep: true, immediate: true },
)
function setPage(value: number) {
	page.value = value
	void load()
}
function link(project: Project) {
	return {
		path: `/curseforge/${project.id}`,
		query: {
			i: props.instanceId || undefined,
			gv: props.preferences.gameVersions?.[0],
			loader: props.preferences.loaders?.[0],
			b: route.fullPath,
		},
	}
}
</script>
<template>
	<section v-if="active" class="mt-4 flex flex-col gap-3" aria-label="CurseForge 搜索结果">
		<h2 class="m-0 text-base font-semibold text-contrast">
			CurseForge
			<span v-if="result" class="font-normal text-secondary"
				>{{ result.pagination.totalCount.toLocaleString() }} 个结果</span
			>
		</h2>
		<div v-if="loading" role="status" aria-label="正在搜索 CurseForge" class="py-6">
			<LoadingIndicator />
		</div>
		<div v-else-if="error" role="alert" class="flex items-center gap-3">
			<span class="text-red">CurseForge 查询失败：{{ error }}</span
			><Button @click="load">重试</Button>
		</div>
		<template v-else>
			<p v-if="note && !result?.data.length" role="status" class="m-0 text-sm text-secondary">
				{{ note }}
			</p>
			<p v-if="!result?.data.length" class="py-6 text-center">
				CurseForge 没有符合当前条件的模组。
			</p>
			<ProjectCardList v-else :layout="layout">
				<ProjectCard
					v-for="project in result.data"
					:key="`curseforge:${project.id}`"
					:layout="layout"
					:link="link(project)"
					:title="
						translatedNames[project.id] ??
						displayModName(project.name, cachedMcmodEntry(mcmodProject(project)))
					"
					@mouseenter="translateName(project)"
					source="CurseForge"
					:icon-url="project.logo?.thumbnailUrl ?? project.logo?.url"
					:author="{
						name: project.authors.map((a) => a.name).join('、'),
						link: project.authors[0]?.url,
					}"
					:summary="project.summary"
					:downloads="project.downloadCount"
					:date-updated="project.dateModified"
					:date-published="project.dateCreated"
					:banner="project.screenshots[0]?.thumbnailUrl ?? project.screenshots[0]?.url"
				>
					<template #actions
						><Button @click="router.push(link(project))">查看版本</Button></template
					>
				</ProjectCard>
			</ProjectCardList>
			<div v-if="pageCount > 1" class="flex justify-end">
				<Pagination :page="page" :count="pageCount" @switch-page="setPage" />
			</div>
		</template>
	</section>
</template>
