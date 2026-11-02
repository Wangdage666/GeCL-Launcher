<script setup lang="ts">
import { ArrowLeftIcon, DownloadIcon, ExternalIcon } from '@modrinth/assets'
import {
	Avatar,
	Button,
	LoadingIndicator,
	Pagination,
	ProgressBar,
	ProjectPageDescription,
} from '@modrinth/ui'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { openUrl } from '@tauri-apps/plugin-opener'
import { configuredXss } from '@modrinth/utils'
import { cachedMcmodEntry, displayModName, mcmodUrl, resolveMcmod } from '@/features/mcmod'
import {
	getDescription,
	getFiles,
	getProject,
	mcmodProject,
	message,
	type File,
} from '@/features/curseforge/api'
import { downloadTasks, stopDownload } from '@/features/curseforge/tasks'
import { injectContentInstall } from '@/providers/content-install'
import { instanceKeys, instanceListQueryOptions } from '@/pages/instance/query-options'

const { install: installContent } = injectContentInstall()
const route = useRoute()
const router = useRouter()
const client = useQueryClient()
const id = computed(() => Number(route.params.id))
const projectQuery = useQuery(
	computed(() => ({
		queryKey: ['curseforge', 'project', id.value],
		queryFn: () => getProject(id.value),
		enabled: Number.isSafeInteger(id.value) && id.value > 0,
		retry: false,
	})),
)
const project = computed(() => projectQuery.data.value)
const instances = useQuery(instanceListQueryOptions())
const availableInstances = computed(() =>
	(instances.data.value ?? []).filter(
		(game) => !game.quarantined && ['forge', 'fabric', 'neoforge', 'quilt'].includes(game.loader),
	),
)
const instanceId = ref(String(route.query.i ?? ''))
watch(
	() => route.query.i,
	(value) => {
		instanceId.value = String(value ?? '')
	},
)
const instance = computed(() =>
	availableInstances.value.find((game) => game.id === instanceId.value),
)
const gameVersion = ref(String(route.query.gv ?? ''))
const loader = ref(String(route.query.loader ?? ''))
const tab = ref(String(route.query.tab ?? 'description'))
const page = ref(1)
const error = ref('')
watch(
	instance,
	(game) => {
		if (game) {
			gameVersion.value = game.game_version
			loader.value = game.loader
		}
	},
	{ immediate: true },
)
watch([gameVersion, loader, id], () => {
	page.value = 1
})
const filesQuery = useQuery(
	computed(() => ({
		queryKey: ['curseforge', 'files', id.value, gameVersion.value, loader.value, page.value],
		queryFn: () =>
			getFiles(id.value, {
				gameVersion: gameVersion.value || undefined,
				loader: loader.value || undefined,
				index: (page.value - 1) * 20,
				pageSize: 20,
			}),
		enabled: !!project.value,
		retry: false,
	})),
)
const files = computed(() => filesQuery.data.value?.data ?? [])
const pageCount = computed(() =>
	Math.max(1, Math.ceil(Math.min(10000, filesQuery.data.value?.pagination.totalCount ?? 0) / 20)),
)
const description = useQuery(
	computed(() => ({
		queryKey: ['curseforge', 'description', id.value],
		queryFn: () => getDescription(id.value),
		enabled: !!project.value && tab.value === 'description',
		retry: false,
	})),
)
const descriptionHtml = computed(() => {
	if (!description.data.value) return ''
	const template = document.createElement('template')
	template.innerHTML = configuredXss.process(description.data.value)
	for (const image of template.content.querySelectorAll('img[src]')) {
		const source = image.getAttribute('src')
		if (!source) continue
		try {
			const url = new URL(source, 'https://www.curseforge.com')
			if (url.protocol === 'http:') url.protocol = 'https:'
			if (url.protocol === 'https:') image.setAttribute('src', url.href)
		} catch {
			image.removeAttribute('src')
		}
	}
	return template.innerHTML
})
const mcmodEntry = ref<ReturnType<typeof cachedMcmodEntry>>(null)
const mcmodOpening = ref(false)
let controller: AbortController | undefined
watch(
	project,
	(value) => {
		controller?.abort()
		if (!value) return
		const target = mcmodProject(value)
		mcmodEntry.value = cachedMcmodEntry(target)
		controller = new AbortController()
		const signal = controller.signal
		void resolveMcmod(target, signal).then((entry) => {
			if (!signal.aborted) mcmodEntry.value = entry
		})
	},
	{ immediate: true },
)
onBeforeUnmount(() => controller?.abort())
async function openMcmod() {
	if (!project.value || mcmodOpening.value) return
	const target = mcmodProject(project.value)
	const signal = controller?.signal
	mcmodOpening.value = true
	error.value = ''
	try {
		const entry =
			mcmodEntry.value ??
			(await resolveMcmod(target, signal, {
				retryMissing: true,
				throwOnError: true,
			}))
		if (signal?.aborted || id.value !== Number(target.id)) return
		if (!entry) {
			error.value = '未找到对应的 MC 百科条目'
			return
		}
		mcmodEntry.value = entry
		await openUrl(mcmodUrl(entry, target.title))
	} catch (value) {
		if (!signal?.aborted) error.value = message(value)
	} finally {
		mcmodOpening.value = false
	}
}
const currentTask = computed(() =>
	[...downloadTasks.value]
		.reverse()
		.find((task) => task.projectId === id.value && task.instanceId === instanceId.value),
)
const busy = computed(() =>
	downloadTasks.value.some(
		(task) =>
			task.instanceId === instanceId.value && ['running', 'canceling'].includes(task.status),
	),
)
async function installAutomatically() {
	if (busy.value) return
	error.value = ''
	try {
		await installContent(`curseforge:${id.value}`, null, instanceId.value || null)
	} catch (errorValue) {
		error.value = message(errorValue)
	}
}
async function installFile(file: File) {
	if (!project.value || !instance.value || busy.value) return
	error.value = ''
	try {
		await installContent(`curseforge:${id.value}`, String(file.id), instanceId.value)
	} catch (errorValue) {
		error.value = message(errorValue)
	}
}

function compatible(file: File) {
	return (
		!!instance.value &&
		file.isAvailable &&
		file.gameVersions.includes(instance.value.game_version) &&
		file.gameVersions.some((l) => l.toLowerCase() === instance.value?.loader)
	)
}
const back = computed(() =>
	typeof route.query.b === 'string' && route.query.b.startsWith('/browse/')
		? route.query.b
		: instanceId.value
			? `/instance/${encodeURIComponent(instanceId.value)}`
			: '/browse/mod',
)
function date(value: string) {
	return new Date(value).toLocaleDateString('zh-CN')
}
</script>
<template>
	<div class="flex flex-col gap-4 p-6">
		<div>
			<Button @click="router.push(back)"><ArrowLeftIcon /> 返回</Button>
		</div>
		<div v-if="projectQuery.isPending.value" role="status"><LoadingIndicator /></div>
		<div v-else-if="projectQuery.error.value" role="alert" class="flex items-center gap-3">
			<span class="text-red">无法读取 CurseForge 项目：{{ message(projectQuery.error.value) }}</span
			><Button @click="projectQuery.refetch()">重试</Button>
		</div>
		<template v-else-if="project">
			<header
				class="flex flex-wrap items-start gap-4 border-0 border-b border-solid border-surface-5 pb-5"
			>
				<Avatar
					:src="project.logo?.thumbnailUrl ?? project.logo?.url"
					:alt="project.name"
					size="80px"
				/>
				<div class="min-w-0 flex-1">
					<h1 class="m-0 mb-2 text-2xl font-bold text-contrast break-words">
						{{ displayModName(project.name, mcmodEntry) }}
					</h1>
					<div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-secondary">
						<span>CurseForge</span
						><span class="flex items-center gap-1"
							><DownloadIcon class="size-4" />{{
								project.downloadCount.toLocaleString('zh-CN')
							}}
							次下载</span
						><span>更新于 {{ date(project.dateModified) }}</span>
					</div>
					<p class="mb-0 mt-2">{{ project.summary }}</p>
				</div>
				<div class="flex flex-wrap gap-2">
					<Button type="colored" color="brand" :disabled="busy" @click="installAutomatically">{{
						instanceId ? '安装' : '添加到游戏版本'
					}}</Button>
					<Button aria-label="MC 百科" :loading="mcmodOpening" @click="openMcmod"
						>{{ mcmodOpening ? '查找中…' : 'MC 百科' }} <ExternalIcon /></Button
					><Button @click="openUrl(project.links.websiteUrl)">官网 <ExternalIcon /></Button>
				</div>
			</header>
			<div class="flex flex-wrap items-center gap-3">
				<span class="text-sm text-secondary">作者</span
				><a
					v-for="author in project.authors"
					:key="author.id"
					:href="author.url"
					target="_blank"
					rel="noopener noreferrer"
					class="flex items-center gap-2 hover:underline"
					><Avatar
						:src="author.avatarUrl ?? undefined"
						:alt="author.name"
						size="24px"
						circle
					/><span>{{ author.name }}</span></a
				>
			</div>
			<p v-if="error" role="alert" class="m-0 text-red">{{ error }}</p>
			<nav aria-label="CurseForge 项目页面" class="flex gap-2">
				<Button
					v-for="item in [
						['description', '介绍'],
						['versions', '版本'],
						['gallery', '图片'],
					]"
					:key="item[0]"
					:type="tab === item[0] ? 'colored' : 'outlined'"
					:color="tab === item[0] ? 'brand' : undefined"
					:aria-pressed="tab === item[0]"
					@click="tab = item[0]"
					>{{ item[1] }}</Button
				>
			</nav>
			<section v-if="tab === 'description'" aria-label="模组介绍" class="max-w-full">
				<div v-if="description.isPending.value" role="status"><LoadingIndicator /></div>
				<ProjectPageDescription v-else-if="descriptionHtml" :description="descriptionHtml" />
				<div v-else class="flex flex-wrap items-center gap-3" role="status">
					<span>{{ description.error.value ? '介绍加载失败' : '暂无介绍' }}</span>
					<Button v-if="description.error.value" @click="description.refetch()">重试</Button>
					<Button @click="openUrl(project.links.websiteUrl)">前往官网 <ExternalIcon /></Button>
				</div>
				<div v-if="project.links.sourceUrl || project.links.wikiUrl" class="flex gap-2 mt-4">
					<Button v-if="project.links.sourceUrl" @click="openUrl(project.links.sourceUrl!)"
						>源代码 <ExternalIcon /></Button
					><Button v-if="project.links.wikiUrl" @click="openUrl(project.links.wikiUrl!)"
						>帮助文档 <ExternalIcon
					/></Button>
				</div>
			</section>
			<section
				v-else-if="tab === 'gallery'"
				aria-label="项目图片"
				class="grid grid-cols-1 gap-4 lg:grid-cols-2"
			>
				<figure v-for="image in project.screenshots" :key="image.url" class="m-0">
					<a :href="image.url" target="_blank" rel="noopener noreferrer"
						><img
							:src="image.url"
							:alt="image.description || image.title"
							class="max-w-full rounded-lg"
							loading="lazy"
					/></a>
					<figcaption class="mt-2 text-sm text-secondary">{{ image.title }}</figcaption>
				</figure>
				<p v-if="!project.screenshots.length">作者还没有上传图片。</p>
			</section>
			<section v-else aria-label="CurseForge 版本列表" class="flex flex-col gap-3">
				<div class="flex flex-wrap items-center gap-3">
					<label class="flex items-center gap-2"
						>安装到<select v-model="instanceId" class="cf-select" :disabled="busy">
							<option value="">仅浏览版本</option>
							<option v-for="game in availableInstances" :key="game.id" :value="game.id">
								{{ game.name }} · {{ game.game_version }} / {{ game.loader }}
							</option>
						</select></label
					>
					<label class="flex items-center gap-2"
						>游戏版本<input
							v-model.trim="gameVersion"
							aria-label="筛选游戏版本"
							class="cf-input"
							placeholder="全部版本"
							:disabled="!!instance || busy"
					/></label>
					<label class="flex items-center gap-2"
						>加载器<select v-model="loader" class="cf-select" :disabled="!!instance || busy">
							<option value="">全部</option>
							<option value="forge">Forge</option>
							<option value="neoforge">NeoForge</option>
							<option value="fabric">Fabric</option>
							<option value="quilt">Quilt</option>
						</select></label
					>
				</div>
				<p v-if="instances.error.value" role="alert" class="text-red">
					读取游戏列表失败，<button class="text-brand underline" @click="instances.refetch()">
						重试
					</button>
				</p>
				<div v-if="filesQuery.isPending.value" role="status"><LoadingIndicator /></div>
				<div v-else-if="filesQuery.error.value" role="alert" class="flex gap-2">
					<span class="text-red">{{ message(filesQuery.error.value) }}</span
					><Button @click="filesQuery.refetch()">重试</Button>
				</div>
				<p v-else-if="!files.length">没有适用于当前游戏版本和加载器的文件。</p>
				<div v-else class="overflow-x-auto">
					<table class="w-full text-left border-collapse">
						<thead>
							<tr class="bg-surface-2">
								<th class="p-3">文件版本</th>
								<th class="p-3">支持版本</th>
								<th class="p-3">下载量</th>
								<th class="p-3">操作</th>
							</tr>
						</thead>
						<tbody>
							<tr
								v-for="file in files"
								:key="file.id"
								class="border-0 border-b border-solid border-surface-5"
							>
								<td class="p-3">
									<div class="font-semibold text-contrast break-all">{{ file.displayName }}</div>
									<div class="text-sm text-secondary">
										{{ date(file.fileDate) }} · {{ (file.fileLength / 1024).toFixed(1) }} KB ·
										{{
											file.releaseType === 1
												? '正式版'
												: file.releaseType === 2
													? '测试版'
													: '预览版'
										}}
									</div>
								</td>
								<td class="p-3 text-sm">{{ file.gameVersions.join('、') }}</td>
								<td class="p-3 tabular-nums">{{ file.downloadCount.toLocaleString('zh-CN') }}</td>
								<td class="p-3">
									<Button
										type="colored"
										color="brand"
										:disabled="busy || !compatible(file)"
										@click="installFile(file)"
										><DownloadIcon />安装</Button
									>
								</td>
							</tr>
						</tbody>
					</table>
				</div>
				<div v-if="pageCount > 1" class="flex justify-end">
					<Pagination :page="page" :count="pageCount" @switch-page="(value) => (page = value)" />
				</div>

				<div
					v-if="currentTask"
					role="status"
					class="flex flex-col gap-2 rounded-lg bg-surface-2 p-3"
				>
					<div class="flex items-center justify-between gap-3">
						<span
							>{{
								currentTask.status === 'failed'
									? currentTask.error
									: currentTask.status === 'canceling'
										? '正在取消并还原…'
										: currentTask.status === 'canceled'
											? '已取消'
											: currentTask.progress.phase
							}}
							· {{ currentTask.progress.name }}</span
						><Button
							v-if="currentTask.status === 'running'"
							size="sm"
							@click="stopDownload(currentTask)"
							>取消</Button
						>
					</div>
					<ProgressBar
						v-if="['running', 'canceling'].includes(currentTask.status)"
						:progress="
							currentTask.progress.total
								? currentTask.progress.completed / currentTask.progress.total
								: 0
						"
						:waiting="!currentTask.progress.total"
					/><Button
						v-if="currentTask.status === 'finished'"
						@click="router.push(`/instance/${encodeURIComponent(instanceId)}`)"
						>查看已安装内容</Button
					>
				</div>
			</section>
		</template>
	</div>
</template>
<style scoped>
.cf-select,
.cf-input {
	min-height: 2.5rem;
	max-width: 28rem;
	border: 1px solid var(--surface-5);
	border-radius: 0.75rem;
	background: var(--surface-2);
	color: var(--color-contrast);
	padding: 0.5rem 0.75rem;
	font: inherit;
}
.cf-input {
	width: 9rem;
}
.cf-select:focus-visible,
.cf-input:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 2px;
}
.cf-select:disabled,
.cf-input:disabled {
	opacity: 0.65;
}
</style>
