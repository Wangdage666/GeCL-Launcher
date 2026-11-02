<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import {
	CheckIcon,
	CompassIcon,
	ExternalIcon,
	GlobeIcon,
	PlusIcon,
	ServerStackIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import type { BrowseInstallContentType, CardAction, ProjectType, Tags } from '@modrinth/ui'
import {
	Button,
	BrowsePageLayout,
	BrowseSidebar,
	commonMessages,
	ContextMenu,
	CreationFlowModal,
	defineMessages,
	formatProjectTypeSentence,
	getLatestMatchingInstallVersion,
	getSelectedInstallPreferences,
	getTargetInstallPreferences,
	injectNotificationManager,
	preferencesDiffer,
	provideBrowseManager,
	requestInstall,
	resolveInstallPlan,
	stripServerRuntimeInstallFilters,
	stripServerRuntimeInstallOverrides,
	useBrowseSearch,
	useDebugLogger,
	useVIntl,
} from '@modrinth/ui'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import type { Ref } from 'vue'
import { computed, onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import type { LocationQuery } from 'vue-router'
import { useRoute, useRouter } from 'vue-router'

import { useAppServerBrowse } from '@/composables/browse/use-app-server-browse'
import { createMergedSearch } from '@/features/browse/merged-search'
import {
	isCurseForge,
	projects as curseforgeProjects,
	searchProjects as searchCurseForge,
} from '@/features/curseforge/search'
import { mcmodProject, getInstalledProjects } from '@/features/curseforge/api'
import { useAppEvent } from '@/composables/use-app-event'
import { useAppSettings } from '@/composables/use-app-settings.ts'
import { cachedMcmodEntry, displayModName, rememberMcmod, resolveMcmod } from '@/features/mcmod'
import { findChineseMods, withChineseMatches } from '@/features/mcmod/chinese-search'
import type { LinkedMcmodEntry } from '@/features/mcmod/parser'
import { get_project, get_search_results_v3, get_version_many } from '@/helpers/cache.js'
import {
	get_installed_project_ids as getInstalledProjectIds,
	getInstanceIconUrl,
	list as listInstances,
} from '@/helpers/instance'
import { get_loader_versions as getLoaderManifest } from '@/helpers/metadata'
import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'
import { get_categories, get_game_versions, get_loaders } from '@/helpers/tags'
import { get_instance_worlds } from '@/helpers/worlds'
import {
	instanceDetailQueryOptions,
	instanceKeys,
	instanceLinkedProjectQueryOptions,
} from '@/pages/instance/query-options'
import { injectContentInstall } from '@/providers/content-install'
import { injectServerInstall } from '@/providers/server-install'
import {
	createServerInstallContent,
	provideServerInstallContent,
} from '@/providers/setup/server-install-content'
import { type BreadcrumbDefinition, injectBreadcrumbManager } from '@/shared/breadcrumbs'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const { installingServerProjects, playServerProject, showAddServerToInstanceModal } =
	injectServerInstall()
const { install: installVersion } = injectContentInstall()
const queryClient = useQueryClient()
const debugLog = useDebugLogger('Browse')

const router = useRouter()
const route = useRoute()
const displayedBrowseRoute = shallowRef(router.currentRoute.value)
watch(
	() => router.currentRoute.value,
	(nextRoute) => {
		if (nextRoute.path.startsWith('/browse/')) {
			displayedBrowseRoute.value = nextRoute
		}
	},
	{ immediate: true },
)
const breadcrumbMessages = defineMessages({
	discoverProjectType: {
		id: 'app.browse.discover-project-type',
		defaultMessage: 'Discover {projectType}',
	},
	discoverServers: {
		id: 'app.browse.discover-servers',
		defaultMessage: 'Discover servers',
	},
})
const breadcrumbLabel = computed(() => {
	const browseRoute = displayedBrowseRoute.value
	if (browseRoute.query.from === 'worlds' || browseRoute.params.projectType === 'server') {
		return formatMessage(breadcrumbMessages.discoverServers)
	}

	return formatMessage(breadcrumbMessages.discoverProjectType, {
		projectType: formatProjectTypeSentence(
			formatMessage,
			String(browseRoute.params.projectType ?? ''),
			2,
		),
	})
})
const appSettings = useAppSettings()
const browseRouteActive = computed(() => route.path.startsWith('/browse/'))
const serverSetupModalRef = ref<InstanceType<typeof CreationFlowModal> | null>(null)
const serverInstallContent = createServerInstallContent({ serverSetupModalRef })
provideServerInstallContent(serverInstallContent)
const {
	serverIdQuery,
	serverFlowFrom,
	isFromWorlds,
	isServerContext,
	isSetupServerContext,
	effectiveServerWorldId,
	serverContextServerData,
	serverContentProjectIds,
	queuedServerInstallRootProjectIds,
	queuedServerInstallProjectIds,
	queuedServerInstallCount,
	selectedServerInstallProjects,
	isInstallingQueuedServerInstalls,
	queuedInstallProgress,
	serverBackUrl,
	serverBackLabel,
	serverBrowseHeading,
	clearQueuedServerInstalls,
	removeQueuedServerInstall,
	flushQueuedServerInstalls,
	discardQueuedServerInstallsAndBack,
	installQueuedServerInstallsAndBack,
	initServerContext,
	watchServerContextChanges,
	searchServerModpacks,
	getServerProjectVersions,
	enforceSetupModpackRoute,
	getQueuedServerInstallPlans,
	setQueuedServerInstallPlans,
	resolveQueuedServerInstallPlan,
	openServerModpackInstallFlow,
	onServerFlowBack,
	handleServerModpackFlowCreate,
	markServerProjectInstalled,
} = serverInstallContent

const initialInstanceId = computed(() => String(route.query.i ?? ''))
const instanceQuery = useQuery(
	computed(() => ({
		...instanceDetailQueryOptions(initialInstanceId.value),
		enabled: !!initialInstanceId.value,
	})),
)
const instance = computed(() => instanceQuery.data.value ?? null)
const linkedInstanceProjectId = computed(() => instance.value?.link?.project_id ?? '')
const linkedInstanceProjectQuery = useQuery(
	computed(() => ({
		...instanceLinkedProjectQueryOptions(linkedInstanceProjectId.value),
		enabled: !!linkedInstanceProjectId.value,
	})),
)
const installedProjectIds: Ref<string[] | null> = ref(null)
const instanceHideInstalled = ref(route.query.ai === 'true')
const newlyInstalled = ref<string[]>([])
const hiddenInstanceProjectIds = ref<Set<string>>(new Set())
const hiddenInstanceProjectIdsInitialized = ref(false)
const isServerInstance = computed(
	() => linkedInstanceProjectQuery.data.value?.minecraft_server != null,
)

const breadcrumbManager = injectBreadcrumbManager()
const instanceBreadcrumbDefinition = {
	slot: 'instance',
	id: () => `instance:${String(displayedBrowseRoute.value.query.i ?? '')}`,
	label: () => instance.value?.name ?? formatMessage(commonMessages.loadingLabel),
	visual: () => ({
		type: 'image' as const,
		src: getInstanceIconUrl(instance.value?.icon_path),
		alt: instance.value?.name,
		tintBy: String(displayedBrowseRoute.value.query.i ?? ''),
	}),
	to: () => {
		const instancePath = `/instance/${encodeURIComponent(
			String(displayedBrowseRoute.value.query.i ?? ''),
		)}`
		return displayedBrowseRoute.value.query.from === 'worlds'
			? `${instancePath}/worlds`
			: instancePath
	},
} satisfies BreadcrumbDefinition
const serversBreadcrumbDefinition = {
	slot: 'root',
	id: 'servers',
	label: () => formatMessage(commonMessages.serversLabel),
	to: '/hosting/manage/',
	visual: { type: 'icon', component: ServerStackIcon },
} satisfies BreadcrumbDefinition
const serverBreadcrumbTo = ref(serverBackUrl.value)
watch(serverBackUrl, (value) => {
	if (route.path.startsWith('/browse/')) {
		serverBreadcrumbTo.value = value
	}
})
const serverBreadcrumbDefinition = {
	slot: 'server',
	id: () => `server:${String(displayedBrowseRoute.value.query.sid ?? '')}`,
	label: () => serverContextServerData.value?.name ?? formatMessage(commonMessages.loadingLabel),
	visual: { type: 'icon', component: ServerStackIcon },
	to: serverBreadcrumbTo,
} satisfies BreadcrumbDefinition
const breadcrumbDefinition = {
	slot: 'browse',
	id: () =>
		`browse:${String(displayedBrowseRoute.value.params.projectType ?? '')}:${String(
			displayedBrowseRoute.value.query.i ?? '',
		)}:${String(displayedBrowseRoute.value.query.sid ?? '')}:${String(
			displayedBrowseRoute.value.query.from ?? '',
		)}`,
	label: breadcrumbLabel,
	to: () => displayedBrowseRoute.value.fullPath,
	visual: { type: 'icon', component: CompassIcon },
} satisfies BreadcrumbDefinition

function syncBreadcrumbs() {
	if (displayedBrowseRoute.value.query.i) {
		const instanceBreadcrumb = breadcrumbManager.reset(instanceBreadcrumbDefinition)
		breadcrumbManager.push(breadcrumbDefinition, { parent: instanceBreadcrumb })
		return
	}

	if (displayedBrowseRoute.value.query.sid) {
		const serversBreadcrumb = breadcrumbManager.reset(serversBreadcrumbDefinition)
		const serverBreadcrumb = breadcrumbManager.push(serverBreadcrumbDefinition, {
			parent: serversBreadcrumb,
		})
		breadcrumbManager.push(breadcrumbDefinition, { parent: serverBreadcrumb })
		return
	}

	breadcrumbManager.reset(breadcrumbDefinition)
}

watch(displayedBrowseRoute, syncBreadcrumbs, { immediate: true, flush: 'sync' })

debugLog('fetching tags (categories, loaders, gameVersions)')
const [categories, loaders, availableGameVersions] = await Promise.all([
	get_categories()
		.catch(handleError)
		.then(ref<Labrinth.Tags.v2.Category[]>),
	get_loaders()
		.catch(handleError)
		.then(ref<Labrinth.Tags.v2.Loader[]>),
	get_game_versions()
		.catch(handleError)
		.then(ref<Labrinth.Tags.v2.GameVersion[]>),
])

const tags: Ref<Tags> = computed(() => ({
	gameVersions: availableGameVersions.value ?? [],
	loaders: loaders.value ?? [],
	categories: categories.value ?? [],
}))

if (isFromWorlds.value && route.params.projectType !== 'server') {
	router.replace({
		path: '/browse/server',
		query: route.query,
	})
}

enforceSetupModpackRoute(route.params.projectType as string | undefined)

const allInstalledIds = computed(
	() => new Set([...newlyInstalled.value, ...(installedProjectIds.value ?? [])]),
)

function syncHiddenInstanceProjectIds() {
	hiddenInstanceProjectIds.value = new Set([
		...(installedProjectIds.value ?? []),
		...newlyInstalled.value,
	])
	hiddenInstanceProjectIdsInitialized.value = true
}

watch(
	installedProjectIds,
	(ids) => {
		if (!ids) return
		if (!hiddenInstanceProjectIdsInitialized.value) {
			syncHiddenInstanceProjectIds()
		}
	},
	{ immediate: true },
)

watchServerContextChanges()

await initInstanceContext()

async function refreshInstalledProjectIds() {
	if (!route.query.i) {
		const instances = await queryClient
			.fetchQuery({
				queryKey: [...instanceKeys.all, 'installed-project-ids'],
				queryFn: listInstances,
				staleTime: 0,
			})
			.catch(handleError)
		if (!instances) return

		const ids = instances
			.map((gameInstance) => gameInstance.link?.project_id)
			.filter((id): id is string => !!id)
		debugLog('installedInstanceProjectIds loaded', { count: ids.length })
		installedProjectIds.value = ids
		return
	}

	if (route.query.from === 'worlds') {
		const targetInstanceId = route.query.i as string
		const worlds = await queryClient
			.fetchQuery({
				queryKey: instanceKeys.installedProjectIds(targetInstanceId, 'worlds'),
				queryFn: () => get_instance_worlds(targetInstanceId),
				staleTime: 0,
			})
			.catch(handleError)
		if (!worlds) return

		const serverProjectIds = worlds
			.filter((w) => w.type === 'server' && 'project_id' in w && w.project_id)
			.map((w) => (w as { project_id: string }).project_id)
		debugLog('installedServerProjectIds loaded', { count: serverProjectIds.length })
		installedProjectIds.value = serverProjectIds
		return
	}

	const targetInstanceId = route.query.i as string
	const ids = await queryClient
		.fetchQuery({
			queryKey: instanceKeys.installedProjectIds(targetInstanceId, 'content'),
			queryFn: () => getInstalledProjectIds(targetInstanceId),
			staleTime: 0,
		})
		.catch(handleError)
	if (!ids) return

	debugLog('installedProjectIds loaded', { count: ids.length })
	installedProjectIds.value = ids
}

async function initInstanceContext() {
	debugLog('initInstanceContext', {
		queryI: route.query.i,
		queryAi: route.query.ai,
		querySid: route.query.sid,
		queryWid: route.query.wid,
		queryFrom: route.query.from,
	})
	await Promise.all([
		initServerContext(),
		refreshInstalledProjectIds(),
		route.query.i ? instanceQuery.suspense().catch(handleError) : Promise.resolve(),
	])

	if (route.query.i) {
		debugLog('instance loaded', {
			name: instance.value?.name,
			loader: instance.value?.loader,
			gameVersion: instance.value?.game_version,
		})

		if (instance.value?.link?.project_id) {
			await linkedInstanceProjectQuery.suspense().catch(handleError)
		}
	}
}

function setHideInstalledModpacks(value: boolean) {
	appSettings.hideInstalledModpacks = value
	getSettings()
		.then((settings) => {
			settings.hide_installed_modpacks = value
			return setSettings(settings)
		})
		.catch(handleError)
}

const hideInstalledModpacks = computed({
	get: () => appSettings.hideInstalledModpacks,
	set: (value: boolean) => setHideInstalledModpacks(value),
})

const instanceFilters = computed(() => {
	const filters = []

	if (instance.value && projectType.value !== 'resourcepack') {
		const isVanillaShader = projectType.value === 'shader' && instance.value.loader === 'vanilla'
		const gameVersion = instance.value.game_version
		if (gameVersion && !isVanillaShader) {
			filters.push({ type: 'game_version', option: gameVersion })
		}

		const platform = instance.value.loader
		const supportedModLoaders = ['fabric', 'forge', 'quilt', 'neoforge']

		if (platform && projectType.value === 'mod' && supportedModLoaders.includes(platform)) {
			filters.push({ type: 'mod_loader', option: platform })
		}
		if (isVanillaShader) {
			filters.push({ type: 'shader_loader', option: 'vanilla' })
		}

		if (isServerInstance.value) {
			filters.push({ type: 'environment', option: 'client' })
		}
	}

	if (
		(instance.value || projectType.value === 'modpack') &&
		(projectType.value === 'modpack' ? hideInstalledModpacks.value : instanceHideInstalled.value) &&
		hiddenInstanceProjectIds.value.size > 0
	) {
		for (const id of hiddenInstanceProjectIds.value) {
			filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
		}
	}

	return filters
})

const serverHideInstalled = ref(false)
const hideSelectedServerInstalls = ref(false)
if (route.query.shi) {
	serverHideInstalled.value = route.query.shi === 'true'
}
const hiddenServerContentProjectIds = ref<Set<string>>(new Set())
const hiddenServerContentProjectIdsInitialized = ref(false)

function syncHiddenServerContentProjectIds() {
	hiddenServerContentProjectIds.value = new Set(serverContentProjectIds.value)
	hiddenServerContentProjectIdsInitialized.value = true
}

watch(
	serverContentProjectIds,
	() => {
		if (!hiddenServerContentProjectIdsInitialized.value) {
			syncHiddenServerContentProjectIds()
		}
	},
	{ immediate: true },
)

const serverContextFilters = computed(() => {
	const filters: { type: string; option: string; negative?: boolean }[] = []
	if (!serverContextServerData.value) return filters
	const pt = projectType.value

	if (pt !== 'modpack') {
		const gameVersion = serverContextServerData.value.mc_version
		if (gameVersion) filters.push({ type: 'game_version', option: gameVersion })

		const platform = serverContextServerData.value.loader?.toLowerCase()
		if (platform && ['fabric', 'forge', 'quilt', 'neoforge'].includes(platform))
			filters.push({ type: 'mod_loader', option: platform })
		if (platform && ['paper', 'purpur'].includes(platform))
			filters.push({ type: 'plugin_loader', option: platform })

		if (pt === 'mod') filters.push({ type: 'environment', option: 'server' })

		if (hideSelectedServerInstalls.value && queuedServerInstallProjectIds.value.size > 0) {
			for (const id of queuedServerInstallProjectIds.value) {
				filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
			}
		}
	}

	if (pt === 'modpack') {
		filters.push(
			{ type: 'environment', option: 'client' },
			{ type: 'environment', option: 'server' },
		)

		if (hideInstalledModpacks.value && hiddenInstanceProjectIds.value.size > 0) {
			for (const id of hiddenInstanceProjectIds.value) {
				filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
			}
		}
	}

	if (serverHideInstalled.value && hiddenServerContentProjectIds.value.size > 0) {
		for (const id of hiddenServerContentProjectIds.value) {
			filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
		}
	}

	return filters
})

const combinedProvidedFilters = computed(() =>
	isServerContext.value ? serverContextFilters.value : instanceFilters.value,
)

const {
	serverPings,
	contextMenuRef,
	updateServerHits,
	getServerModpackContent,
	getServerCardActions,
	handleRightClick,
} = useAppServerBrowse({
	instance,
	isFromWorlds,
	allInstalledIds,
	newlyInstalled,
	installingServerProjects,
	playServerProject,
	showAddServerToInstanceModal,
	handleError,
	router,
})

const offline = ref(!navigator.onLine)
const handleOffline = () => {
	debugLog('went offline')
	offline.value = true
}
const handleOnline = () => {
	debugLog('went online')
	offline.value = false
}
window.addEventListener('offline', handleOffline)
window.addEventListener('online', handleOnline)

onBeforeUnmount(() => {
	window.removeEventListener('offline', handleOffline)
	window.removeEventListener('online', handleOnline)
})

const messages = defineMessages({
	addServersToInstance: {
		id: 'app.browse.add-servers-to-instance',
		defaultMessage: 'Adding server to instance',
	},
	projectActionsLabel: {
		id: 'app.browse.project-actions.label',
		defaultMessage: 'Project actions',
	},
	addToAnInstance: {
		id: 'app.browse.add-to-an-instance',
		defaultMessage: 'Add to an instance',
	},
	environmentProvidedByServer: {
		id: 'search.filter.locked.server-environment.title',
		defaultMessage: 'Only client-side mods can be added to the server instance',
	},
	gameVersionProvidedByInstance: {
		id: 'search.filter.locked.instance-game-version.title',
		defaultMessage: 'Game version is provided by the instance',
	},
	hideAddedServers: {
		id: 'app.browse.hide-added-servers',
		defaultMessage: 'Hide servers already added',
	},
	hideInstalledModpacks: {
		id: 'app.browse.hide-installed-modpacks',
		defaultMessage: 'Hide already installed',
	},
	installingToServer: {
		id: 'app.browse.server.installing',
		defaultMessage: '正在安装',
	},
	backToInstance: {
		id: 'app.browse.back-to-instance',
		defaultMessage: 'Back to instance',
	},
	serverInstanceContentWarning: {
		id: 'app.browse.server-instance-content-warning',
		defaultMessage:
			'Adding content may prevent you from joining this server. Any content you add will be removed when the managed server content is updated.',
	},
	modLoaderProvidedByInstance: {
		id: 'search.filter.locked.instance-loader.title',
		defaultMessage: 'Loader is provided by the instance',
	},
	modpacksProjectType: {
		id: 'app.browse.project-type.modpacks',
		defaultMessage: 'Modpacks',
	},
	modsProjectType: { id: 'app.browse.project-type.mods', defaultMessage: 'Mods' },
	resourcePacksProjectType: {
		id: 'app.browse.project-type.resource-packs',
		defaultMessage: 'Resource Packs',
	},
	dataPacksProjectType: {
		id: 'app.browse.project-type.data-packs',
		defaultMessage: 'Data Packs',
	},
	shadersProjectType: { id: 'app.browse.project-type.shaders', defaultMessage: 'Shaders' },
	serversProjectType: { id: 'app.browse.project-type.servers', defaultMessage: 'Servers' },
	providedByInstance: {
		id: 'search.filter.locked.instance',
		defaultMessage: 'Provided by the instance',
	},
	syncFilterButton: {
		id: 'search.filter.locked.instance.sync',
		defaultMessage: 'Sync with instance',
	},
})

const projectType = ref<ProjectType>(route.params.projectType as ProjectType)

function resetInstanceContext() {
	debugLog('instance context removed, resetting')
	installedProjectIds.value = null
	instanceHideInstalled.value = false
	newlyInstalled.value = []
	hiddenInstanceProjectIds.value = new Set()
	hiddenInstanceProjectIdsInitialized.value = false
	isServerInstance.value = false
	browseBreadcrumb.reset()
	void refreshInstalledProjectIds()
}

watch(
	() => route.params.projectType as ProjectType,
	async (newType) => {
		if (!browseRouteActive.value) {
			return
		}
		if (isSetupServerContext.value) {
			enforceSetupModpackRoute(newType)
			if (newType !== 'modpack') return
		}

		if (!newType || newType === projectType.value) return

		debugLog('projectType route param changed', { from: projectType.value, to: newType })
		projectType.value = newType
	},
)

watch(
	() => route.query.i,
	async (nextInstanceId, previousInstanceId) => {
		if (!route.path.startsWith('/browse') || nextInstanceId === previousInstanceId) return
		if (!nextInstanceId) {
			resetInstanceContext()
			return
		}

		installedProjectIds.value = null
		hiddenInstanceProjectIdsInitialized.value = false
		await Promise.all([instanceQuery.suspense().catch(handleError), refreshInstalledProjectIds()])
		if (instance.value?.link?.project_id) {
			await linkedInstanceProjectQuery.suspense().catch(handleError)
		}
	},
)

const selectableProjectTypes = computed(() => {
	let dataPacks = false,
		mods = false,
		modpacks = false

	if (instance.value) {
		if (
			availableGameVersions.value &&
			availableGameVersions.value.findIndex((x) => x.version === instance.value?.game_version) <=
				availableGameVersions.value.findIndex((x) => x.version === '1.13') &&
			!isServerInstance.value
		) {
			dataPacks = true
		}

		if (instance.value.loader !== 'vanilla') {
			mods = true
		}
	} else {
		dataPacks = true
		mods = true
		modpacks = true
	}

	const params: LocationQuery = {}

	if (route.query.i) params.i = route.query.i
	if (route.query.ai) params.ai = route.query.ai
	if (route.query.from) params.from = route.query.from
	if (route.query.sid) params.sid = route.query.sid
	if (effectiveServerWorldId.value) params.wid = effectiveServerWorldId.value

	const queryString = new URLSearchParams(params as Record<string, string>).toString()
	const suffix = queryString ? `?${queryString}` : ''

	if (isSetupServerContext.value) {
		return [
			{ label: formatMessage(messages.modpacksProjectType), href: `/browse/modpack${suffix}` },
		]
	}

	if (isFromWorlds.value) {
		return [{ label: formatMessage(messages.serversProjectType), href: `/browse/server${suffix}` }]
	}

	return [
		{
			label: formatMessage(messages.modpacksProjectType),
			href: `/browse/modpack${suffix}`,
			shown: modpacks,
		},
		{ label: formatMessage(messages.modsProjectType), href: `/browse/mod${suffix}`, shown: mods },
		{
			label: formatMessage(messages.resourcePacksProjectType),
			href: `/browse/resourcepack${suffix}`,
		},
		{
			label: formatMessage(messages.dataPacksProjectType),
			href: `/browse/datapack${suffix}`,
			shown: dataPacks,
		},
		{ label: formatMessage(messages.shadersProjectType), href: `/browse/shader${suffix}` },
		{
			label: formatMessage(messages.serversProjectType),
			href: `/browse/server${suffix}`,
			shown: !instance.value,
		},
	]
})

const installContext = computed(() => {
	if (isServerContext.value && serverContextServerData.value) {
		return {
			name: serverContextServerData.value.name,
			loader: serverContextServerData.value.loader ?? '',
			gameVersion: serverContextServerData.value.mc_version ?? '',
			serverId: serverIdQuery.value,
			upstream: serverContextServerData.value.upstream,
			iconSrc: null as string | null,
			isMedal: serverContextServerData.value.is_medal,
			backUrl: serverBackUrl.value,
			backLabel: serverBackLabel.value,
			heading: serverBrowseHeading.value,
			queuedCount: queuedServerInstallCount.value,
			selectedProjects: selectedServerInstallProjects.value,
			isInstallingSelected: isInstallingQueuedServerInstalls.value,
			skipNonEssentialWarnings: appSettings.skipNonEssentialWarnings,
			installProgress: queuedInstallProgress.value,
			clearQueued: clearQueuedServerInstalls,
			clearSelected: clearQueuedServerInstalls,
			onBack: flushQueuedServerInstalls,
			discardSelectedAndBack: discardQueuedServerInstallsAndBack,
			installSelected: installQueuedServerInstallsAndBack,
		}
	}
	if (instance.value) {
		return {
			name: instance.value.name,
			loader: instance.value.loader,
			gameVersion: instance.value.game_version,
			iconSrc: getInstanceIconUrl(instance.value.icon_path),
			backUrl: `/instance/${encodeURIComponent(instance.value.id)}${isFromWorlds.value ? '/worlds' : ''}`,
			backLabel: formatMessage(messages.backToInstance),
			heading: formatMessage(
				isFromWorlds.value ? messages.addServersToInstance : commonMessages.installingContentLabel,
			),
			warning:
				isServerInstance.value && instance.value.loader !== 'vanilla' && !isFromWorlds.value
					? formatMessage(messages.serverInstanceContentWarning)
					: undefined,
		}
	}
	return null
})

const installingProjectIds = ref<Set<string>>(new Set())

function setProjectInstalling(projectId: string, installing: boolean) {
	const next = new Set(installingProjectIds.value)
	if (installing) {
		next.add(projectId)
	} else {
		next.delete(projectId)
	}
	installingProjectIds.value = next
}

const serverInstallQueue = {
	get: getQueuedServerInstallPlans,
	set: setQueuedServerInstallPlans,
}

function getCurrentSelectedInstallPreferences(projectTypeValue: string) {
	return getSelectedInstallPreferences({
		contentType: projectTypeValue,
		selectedFilters: searchState.currentFilters.value,
		providedFilters: combinedProvidedFilters.value,
		overriddenProvidedFilterTypes: searchState.overriddenProvidedFilterTypes.value,
	})
}

function getServerInstallTargetPreferences(contentType: BrowseInstallContentType) {
	return getTargetInstallPreferences(
		{
			gameVersion: serverContextServerData.value?.mc_version,
			loader: serverContextServerData.value?.loader,
		},
		contentType,
	)
}

function getInstanceInstallTargetPreferences(projectTypeValue: string) {
	return getTargetInstallPreferences(
		{
			gameVersion: instance.value?.game_version,
			loader: instance.value?.loader,
		},
		projectTypeValue,
	)
}

async function getInstallProjectVersions(projectId: string) {
	const project = await get_project(projectId, 'must_revalidate')
	return (await get_version_many(
		project.versions,
		'must_revalidate',
	)) as Labrinth.Versions.v2.Version[]
}

async function chooseInstanceInstallVersion(
	project: Labrinth.Search.v3.ResultSearchProject,
	projectTypeValue: string,
) {
	const targetInstance = instance.value
	if (!targetInstance) {
		return { versionId: null as string | null }
	}

	const selectedPreferences = getCurrentSelectedInstallPreferences(projectTypeValue)
	const targetPreferences = getInstanceInstallTargetPreferences(projectTypeValue)
	if (!preferencesDiffer(selectedPreferences, targetPreferences)) {
		return { versionId: null as string | null }
	}

	const selectedVersion = getLatestMatchingInstallVersion(
		await getInstallProjectVersions(project.project_id),
		selectedPreferences,
	)

	if (!selectedVersion) {
		return { versionId: null as string | null }
	}

	return { versionId: selectedVersion.id }
}

async function chooseFilterMatchingInstallVersion(
	project: Labrinth.Search.v3.ResultSearchProject,
	projectTypeValue: string,
) {
	const plan = await resolveInstallPlan({
		project: {
			project_id: project.project_id,
			title: project.name,
			icon_url: project.icon_url,
		},
		contentType: projectTypeValue as BrowseInstallContentType,
		selectedFilters: searchState.currentFilters.value,
		providedFilters: combinedProvidedFilters.value,
		overriddenProvidedFilterTypes: searchState.overriddenProvidedFilterTypes.value,
		targetPreferences: {},
		getProjectVersions: getInstallProjectVersions,
	})

	return { versionId: plan.versionId }
}

function getCardActions(
	result: Labrinth.Search.v3.ResultSearchProject,
	currentProjectType: string,
): CardAction[] {
	if (isCurseForge(result.project_id)) {
		const id = result.project_id
		const installed = newlyInstalled.value.includes(id) || installedCurseForgeIds.value.has(id)
		const installing = installingProjectIds.value.has(id)
		return [
			{
				key: 'install',
				label: formatMessage(
					installing
						? commonMessages.installingLabel
						: installed
							? commonMessages.installedLabel
							: instance.value
								? commonMessages.installButton
								: messages.addToAnInstance,
				),
				icon: installing ? SpinnerIcon : installed ? CheckIcon : PlusIcon,
				iconClass: installing ? 'animate-spin' : undefined,
				disabled: installed || installing,
				color: 'brand',
				type: 'outlined',
				onClick: async () => {
					setProjectInstalling(id, true)
					try {
						await installVersion(id, null, instance.value?.id, 'SearchCard', (version, ids) => {
							setProjectInstalling(id, false)
							if (version) onSearchResultsInstalled(ids ?? [id])
						})
					} catch (error) {
						setProjectInstalling(id, false)
						handleError(error)
					}
				},
			},
		]
	}
	if (currentProjectType === 'server') {
		return getServerCardActions(result)
	}

	const projectResult = result as Labrinth.Search.v3.ResultSearchProject & {
		installed?: boolean
		installing?: boolean
	}
	const isInstalled =
		projectResult.installed ||
		allInstalledIds.value.has(projectResult.project_id || '') ||
		serverContentProjectIds.value.has(projectResult.project_id || '') ||
		serverContextServerData.value?.upstream?.project_id === projectResult.project_id
	const isInstalling = installingProjectIds.value.has(projectResult.project_id)
	const showAsInstalled = isInstalled && currentProjectType !== 'modpack'

	if (
		isServerContext.value &&
		['modpack', 'mod', 'plugin', 'datapack'].includes(currentProjectType)
	) {
		const isQueued = queuedServerInstallProjectIds.value.has(projectResult.project_id)
		const isQueuedRoot = queuedServerInstallRootProjectIds.value.has(projectResult.project_id)
		const isInstallingSelection = isInstallingQueuedServerInstalls.value
		const validatingInstall =
			isInstalling && currentProjectType !== 'modpack' && !isInstallingSelection
		const installLabel = showAsInstalled
			? commonMessages.installedLabel
			: isQueued
				? isInstalling || isInstallingSelection
					? validatingInstall
						? commonMessages.validatingLabel
						: messages.installingToServer
					: commonMessages.selectedLabel
				: isInstalling || isInstallingSelection
					? validatingInstall
						? commonMessages.validatingLabel
						: messages.installingToServer
					: commonMessages.installButton
		return [
			{
				key: 'install',
				label: formatMessage(installLabel),
				icon:
					isInstalling || isInstallingSelection
						? SpinnerIcon
						: isQueued || showAsInstalled
							? CheckIcon
							: PlusIcon,
				iconClass: isInstalling || isInstallingSelection ? 'animate-spin' : undefined,
				disabled:
					showAsInstalled || isInstalling || isInstallingSelection || (isQueued && !isQueuedRoot),
				color: isQueued && !isInstalling && !isInstallingSelection ? 'green' : 'brand',
				type: 'outlined',
				onClick: async () => {
					if (isQueuedRoot) {
						removeQueuedServerInstall(projectResult.project_id)
						return
					}
					if (isQueued) return

					const contentType = currentProjectType as BrowseInstallContentType
					const isModpack = contentType === 'modpack'
					const shouldShowInstalling = isModpack || !isQueued
					if (shouldShowInstalling) {
						setProjectInstalling(projectResult.project_id, true)
					}
					try {
						const plan = await requestInstall({
							project: projectResult,
							contentType,
							mode: isModpack ? 'immediate' : 'queue',
							selectedFilters: isModpack
								? []
								: stripServerRuntimeInstallFilters(searchState.currentFilters.value),
							providedFilters: isModpack ? [] : combinedProvidedFilters.value,
							overriddenProvidedFilterTypes: isModpack
								? []
								: stripServerRuntimeInstallOverrides(
										searchState.overriddenProvidedFilterTypes.value,
									),
							targetPreferences: getServerInstallTargetPreferences(contentType),
							getProjectVersions: getInstallProjectVersions,
							queue: serverInstallQueue,
							install: (plan) =>
								openServerModpackInstallFlow({
									projectId: plan.projectId,
									versionId: plan.versionId,
									name: plan.project.name,
									iconUrl: plan.project.icon_url ?? undefined,
								}),
						})
						if (!isModpack) await resolveQueuedServerInstallPlan(plan)
					} catch (err) {
						if (!isModpack) removeQueuedServerInstall(projectResult.project_id)
						handleError(err as Error)
					} finally {
						if (shouldShowInstalling) {
							setProjectInstalling(projectResult.project_id, false)
						}
					}
				},
			},
		]
	}

	const isModpack = projectResult.project_types?.includes('modpack')
	const shouldUseInstallIcon = !!instance.value || isModpack

	return [
		{
			key: 'install',
			label: formatMessage(
				isInstalling
					? messages.installingToServer
					: showAsInstalled
						? commonMessages.installedLabel
						: shouldUseInstallIcon
							? commonMessages.installButton
							: messages.addToAnInstance,
			),
			icon: isInstalling ? SpinnerIcon : showAsInstalled ? CheckIcon : PlusIcon,
			iconClass: isInstalling ? 'animate-spin' : undefined,
			disabled: showAsInstalled || isInstalling,
			color: 'brand',
			type: 'outlined',
			onClick: async () => {
				setProjectInstalling(projectResult.project_id, true)
				try {
					const selectedInstall = instance.value
						? await chooseInstanceInstallVersion(projectResult, currentProjectType)
						: isModpack
							? await chooseFilterMatchingInstallVersion(projectResult, currentProjectType)
							: { versionId: null as string | null }
					if (selectedInstall === null) {
						setProjectInstalling(projectResult.project_id, false)
						return
					}
					const selectedPreferences = getCurrentSelectedInstallPreferences(currentProjectType)
					await installVersion(
						projectResult.project_id,
						selectedInstall.versionId,
						instance.value ? instance.value.id : null,
						'SearchCard',
						(versionId, installedProjectIds) => {
							setProjectInstalling(projectResult.project_id, false)
							if (versionId) {
								onSearchResultsInstalled(installedProjectIds ?? [projectResult.project_id])
							}
						},
						(profile) => {
							router.push(`/instance/${profile}`)
						},
						{
							preferredLoader: instance.value?.loader ?? selectedPreferences.loaders?.[0],
							preferredGameVersion:
								instance.value?.game_version ?? selectedPreferences.gameVersions?.[0],
						},
					)
				} catch (err) {
					setProjectInstalling(projectResult.project_id, false)
					handleError(err)
				}
			},
		},
	]
}

function onSearchResultInstalled(id: string) {
	if (isServerContext.value) {
		markServerProjectInstalled(id)
		return
	}
	if (!newlyInstalled.value.includes(id)) {
		newlyInstalled.value = [...newlyInstalled.value, id]
	}
}

function onSearchResultsInstalled(ids: string[]) {
	if (isServerContext.value) {
		for (const id of ids) {
			markServerProjectInstalled(id)
		}
		return
	}
	newlyInstalled.value = Array.from(new Set([...newlyInstalled.value, ...ids]))
}

const browseSource = ref<'all' | 'modrinth' | 'curseforge'>('all')
const sourceErrors = ref<string[]>([])
const mergedSearch = createMergedSearch<Labrinth.Search.v3.ResultSearchProject>(
	(hit) => hit.project_id,
)
const installedCurseForge = useQuery({
	queryKey: ['instances', 'curseforge-installed'],
	queryFn: getInstalledProjects,
	enabled: computed(() => projectType.value === 'mod'),
})
const installedCurseForgeIds = computed(
	() =>
		new Set(
			(installedCurseForge.data.value?.[instance.value?.id ?? ''] ?? []).map(
				(id) => `curseforge:${id}`,
			),
		),
)
async function retrySources() {
	mergedSearch.clear()
	await searchState.refreshSearch()
}
async function search(requestParams: string) {
	chineseSearchController?.abort()
	const controller = new AbortController()
	chineseSearchController = controller
	chineseSearchNote.value = ''
	sourceErrors.value = []
	if (projectType.value !== 'mod' || isServerContext.value)
		return searchModrinth(requestParams, controller)
	const params = new URLSearchParams(requestParams.replace(/^\?/, ''))
	const offset = Number(params.get('offset') ?? 0)
	const limit = Number(params.get('limit') ?? 20)
	const sort = params.get('index') ?? 'relevance'
	const query = params.get('query') ?? ''
	const preferences = getCurrentSelectedInstallPreferences('mod')
	const entries = /\p{Script=Han}/u.test(query)
		? await findChineseMods(query, controller.signal).catch(() => [])
		: []
	const feeds = []
	if (browseSource.value !== 'curseforge')
		feeds.push({
			name: 'Modrinth',
			load: async (start: number, size: number) => {
				const next = new URLSearchParams(params)
				next.set('offset', String(start))
				next.set('limit', String(size))
				const result = await searchModrinth(`?${next}`, controller, entries)
				return { items: result.projectHits, total: result.total_hits }
			},
		})
	if (browseSource.value !== 'modrinth')
		feeds.push({
			name: 'CurseForge',
			load: async (start: number, size: number) => {
				if ((preferences.gameVersions?.length ?? 0) > 1 || (preferences.loaders?.length ?? 0) > 1)
					throw new Error('请一次选择一个游戏版本和一种加载器。')
				const result = await searchCurseForge(
					{
						query,
						index: start,
						pageSize: size,
						gameVersion: preferences.gameVersions?.[0],
						loader: preferences.loaders?.[0],
						sort,
					},
					entries,
				)
				return { ...result, consumed: result.items.length }
			},
		})
	params.delete('offset')
	params.delete('limit')
	const compare =
		sort === 'downloads'
			? (a: Labrinth.Search.v3.ResultSearchProject, b: Labrinth.Search.v3.ResultSearchProject) =>
					b.downloads - a.downloads
			: ['newest', 'updated'].includes(sort)
				? (a: Labrinth.Search.v3.ResultSearchProject, b: Labrinth.Search.v3.ResultSearchProject) =>
						Date.parse(sort === 'newest' ? b.date_created : b.date_modified) -
						Date.parse(sort === 'newest' ? a.date_created : a.date_modified)
				: undefined
	const result = await mergedSearch.page({
		key: `${browseSource.value}:${params}:${JSON.stringify(preferences)}:${instanceHideInstalled.value ? [...installedCurseForgeIds.value].sort().join(',') : ''}`,
		offset,
		limit,
		feeds,
		signal: controller.signal,
		compare,
		include: (hit) =>
			!instanceHideInstalled.value || !installedCurseForgeIds.value.has(hit.project_id),
	})
	if (!controller.signal.aborted) sourceErrors.value = result.errors

	return { projectHits: result.items, serverHits: [], total_hits: result.total, per_page: limit }
}
const chineseSearchNote = ref('')
let chineseSearchController: AbortController | undefined
onBeforeUnmount(() => chineseSearchController?.abort())
async function searchModrinth(
	requestParams: string,
	controller: AbortController,
	knownEntries?: LinkedMcmodEntry[],
) {
	debugLog('searching v3', requestParams)
	const isServer = projectType.value === 'server'
	let chineseEntries: LinkedMcmodEntry[] = []
	const query = new URLSearchParams(requestParams.replace(/^\?/, '')).get('query') ?? ''
	if (projectType.value === 'mod' && /\p{Script=Han}/u.test(query)) {
		try {
			chineseEntries = knownEntries ?? (await findChineseMods(query, controller.signal))
			if (!chineseEntries.length)
				chineseSearchNote.value = '暂未找到这个中文名对应的 Modrinth 项目，可尝试英文名。'
			requestParams = withChineseMatches(requestParams, chineseEntries)
		} catch (error) {
			if (!controller.signal.aborted)
				chineseSearchNote.value = '中文名称查询暂时不可用，可稍后重试或使用英文名。'
		}
	}
	if (controller.signal.aborted)
		return { projectHits: [], serverHits: [], total_hits: 0, per_page: 20 }

	const rawResults = await queryClient.fetchQuery({
		queryKey: ['search', 'v3', requestParams],
		queryFn: () =>
			get_search_results_v3(requestParams, 'must_revalidate') as Promise<{
				result: Labrinth.Search.v3.SearchResults & {
					hits: (Labrinth.Search.v3.ResultSearchProject & { installed?: boolean })[]
				}
			} | null>,
		staleTime: 30_000,
	})

	if (!rawResults) {
		return {
			projectHits: [],
			serverHits: [],
			total_hits: 0,
			per_page: 20,
		}
	}

	for (const hit of rawResults.result.hits) {
		for (const identifier of [hit.project_id, hit.slug]) {
			if (identifier) {
				queryClient.setQueryData(['projects', 'summary', identifier], hit)
			}
		}
	}

	if (isServer) {
		const hits = rawResults.result.hits ?? []
		updateServerHits(hits)
		return {
			projectHits: [],
			serverHits: hits,
			total_hits: rawResults.result.total_hits ?? 0,
			per_page: rawResults.result.hits_per_page,
		}
	}

	const hits = rawResults.result.hits.map((hit) => {
		if (projectType.value === 'mod') originalModTitles.set(hit.project_id, hit.name)
		const mapped: Labrinth.Search.v3.ResultSearchProject & { installed?: boolean } = {
			...hit,
		}
		const chinese = chineseEntries.find(
			(entry) =>
				entry.identifiers.includes(hit.project_id) || entry.identifiers.includes(hit.slug ?? ''),
		)
		if (chinese) rememberMcmod({ ...hit, title: hit.name }, chinese)
		if (projectType.value === 'mod')
			mapped.name = displayModName(hit.name, cachedMcmodEntry({ ...hit, title: hit.name }))

		if (instance.value || isServerContext.value || projectType.value === 'modpack') {
			const installedIds =
				isServerContext.value && projectType.value !== 'modpack'
					? serverContentProjectIds.value
					: new Set([...newlyInstalled.value, ...(installedProjectIds.value ?? [])])
			mapped.installed = installedIds.has(hit.project_id)
		}

		return mapped
	})

	return {
		projectHits: hits,
		serverHits: [],
		total_hits: rawResults.result.total_hits,
		per_page: rawResults.result.hits_per_page,
	}
}

watch(browseSource, () => {
	searchState.currentPage.value = 1
	if (
		browseSource.value !== 'modrinth' &&
		searchState.effectiveCurrentSortType.value.name === 'follows'
	)
		searchState.effectiveCurrentSortType.value = searchState.effectiveSortTypes.value[0]
	void searchState.refreshSearch()
})

const lockedFilterMessages = computed(() => ({
	gameVersion: formatMessage(messages.gameVersionProvidedByInstance),
	modLoader: formatMessage(messages.modLoaderProvidedByInstance),
	environment: formatMessage(messages.environmentProvidedByServer),
	syncButton: formatMessage(messages.syncFilterButton),
	providedBy: formatMessage(messages.providedByInstance),
}))

const searchState = useBrowseSearch({
	projectType,
	tags,
	active: browseRouteActive,
	providedFilters: combinedProvidedFilters,
	search,
	persistentQueryParams: ['i', 'ai', 'shi', 'sid', 'wid', 'from'],
	getExtraQueryParams: () => ({
		sid: serverIdQuery.value || undefined,
		wid: effectiveServerWorldId.value || undefined,
		ai: instanceHideInstalled.value ? 'true' : undefined,
		shi: serverHideInstalled.value ? 'true' : undefined,
	}),
})

watch(
	[
		() => searchState.query.value,
		() =>
			searchState.isServerType.value
				? searchState.serverCurrentFilters.value
				: searchState.currentFilters.value,
		() => projectType.value,
	],
	() => {
		if (isServerContext.value && projectType.value !== 'modpack') {
			syncHiddenServerContentProjectIds()
		} else if (instance.value || projectType.value === 'modpack') {
			syncHiddenInstanceProjectIds()
		}
	},
	{ deep: true },
)

watch(queuedServerInstallCount, (count) => {
	if (count === 0) {
		hideSelectedServerInstalls.value = false
	}
})

if (instance.value?.game_version) {
	const gv = instance.value.game_version
	const alreadyHasGv = searchState.serverCurrentFilters.value.some(
		(f) => f.type === 'server_game_version' && f.option === gv,
	)
	if (!alreadyHasGv) {
		searchState.serverCurrentFilters.value.push({ type: 'server_game_version', option: gv })
	}
}

void searchState.refreshSearch()

useAppEvent('instance', async (event) => {
	if (event.event === 'created' || event.event === 'removed') {
		if (!route.query.i) {
			await refreshInstalledProjectIds()
			if (projectType.value === 'modpack') {
				if (event.event === 'removed') {
					syncHiddenInstanceProjectIds()
				}
				await searchState.refreshSearch()
			}
		}
	}

	if (instance.value && event.instance_id === instance.value.id && event.event === 'synced') {
		await refreshInstalledProjectIds()
		await searchState.refreshSearch()
	}
})

function getProjectBrowseQuery() {
	if (!browseRouteActive.value) {
		return undefined
	}
	if (!installContext.value) return undefined
	return {
		...route.query,
		b: route.fullPath,
	}
}

const originalModTitles = new Map<string, string>()
watch(
	() =>
		[projectType.value, ...searchState.projectHits.value.map((hit) => hit.project_id)].join('|'),
	(_key, _previous, onCleanup) => {
		const hits = searchState.projectHits.value
		if (projectType.value !== 'mod') return
		const controller = new AbortController()
		onCleanup(() => controller.abort())
		const entries = hits.map((hit) => ({
			...hit,
			title: originalModTitles.get(hit.project_id) ?? hit.name,
		}))
		let next = 0
		async function worker() {
			while (next < entries.length && !controller.signal.aborted) {
				const project = entries[next++]
				const cfProject = curseforgeProjects.get(project.project_id)
				const target = cfProject ? mcmodProject(cfProject) : project
				const entry = cachedMcmodEntry(target) ?? (await resolveMcmod(target, controller.signal))
				if (!entry || controller.signal.aborted) continue
				const current = searchState.projectHits.value
				const display = displayModName(project.title, entry)
				const hit = current.find((item) => item.project_id === project.project_id)
				if (!hit || hit.name === display) continue
				// A shallow ref update makes the verified Chinese title reactive in the shared cards.
				searchState.projectHits.value = current.map((item) =>
					item.project_id === project.project_id ? { ...item, name: display } : item,
				)
			}
		}
		void worker()
		void worker()
	},
)

const advancedFiltersCollapsed = computed({
	get: () => appSettings.advancedFiltersCollapsed,
	set: (value) => {
		appSettings.advancedFiltersCollapsed = value
		getSettings()
			.then((settings) => {
				settings.advanced_filters_collapsed = value
				return setSettings(settings)
			})
			.catch(handleError)
	},
})

const dismissedPhotosensitivityFilterWarning = computed({
	get: () => appSettings.dismissedPhotosensitivityFilterWarning,
	set: (value) => {
		appSettings.dismissedPhotosensitivityFilterWarning = value
		getSettings()
			.then((settings) => {
				settings.dismissed_photosensitivity_filter_warning = value
				return setSettings(settings)
			})
			.catch(handleError)
	},
})

provideBrowseManager({
	tags,
	projectType,
	...searchState,
	effectiveSortTypes: computed(() =>
		projectType.value === 'mod' && browseSource.value !== 'modrinth'
			? searchState.effectiveSortTypes.value.filter((type) => type.name !== 'follows')
			: searchState.effectiveSortTypes.value,
	),
	getProjectSource: (result) =>
		projectType.value === 'mod'
			? isCurseForge(result.project_id)
				? 'CurseForge'
				: 'Modrinth'
			: undefined,
	getProjectAuthorLink: (result) =>
		isCurseForge(result.project_id)
			? (curseforgeProjects.get(result.project_id)?.authors[0]?.url ?? '')
			: undefined,
	getProjectFollowers: (result) => (isCurseForge(result.project_id) ? undefined : result.follows),
	advancedFiltersCollapsed,
	dismissedPhotosensitivityFilterWarning,
	getProjectLink: (result: Labrinth.Search.v3.ResultSearchProject) => ({
		path: isCurseForge(result.project_id)
			? `/curseforge/${result.project_id.split(':')[1]}`
			: `/project/${result.project_id ?? result.slug}`,
		query: getProjectBrowseQuery(),
	}),
	getServerProjectLink: (result: Labrinth.Search.v3.ResultSearchProject) => ({
		path: `/project/${result.slug ?? result.project_id}`,
		query: getProjectBrowseQuery(),
	}),
	selectableProjectTypes,
	showProjectTypeTabs: computed(() => !isServerContext.value),
	variant: 'app',
	getCardActions,
	installContext,
	providedFilters: combinedProvidedFilters,
	hideInstalled: computed({
		get: () => {
			if (projectType.value === 'modpack') return hideInstalledModpacks.value
			if (isServerContext.value) return serverHideInstalled.value
			return instanceHideInstalled.value
		},
		set: (val: boolean) => {
			if (projectType.value === 'modpack') {
				hideInstalledModpacks.value = val
				if (val) syncHiddenInstanceProjectIds()
				return
			}
			if (isServerContext.value) {
				serverHideInstalled.value = val
				if (val) syncHiddenServerContentProjectIds()
			} else {
				instanceHideInstalled.value = val
				if (val) syncHiddenInstanceProjectIds()
			}
		},
	}),
	showHideInstalled: computed(
		() =>
			projectType.value === 'modpack' ||
			(isServerContext.value && projectType.value !== 'modpack') ||
			!!instance.value,
	),
	hideInstalledLabel: computed(() =>
		formatMessage(
			isFromWorlds.value
				? messages.hideAddedServers
				: projectType.value === 'modpack'
					? messages.hideInstalledModpacks
					: commonMessages.hideInstalledContentLabel,
		),
	),
	hideSelected: hideSelectedServerInstalls,
	showHideSelected: computed(
		() =>
			isServerContext.value &&
			projectType.value !== 'modpack' &&
			queuedServerInstallCount.value > 0,
	),
	hideSelectedLabel: computed(() => formatMessage(commonMessages.hideSelectedContentLabel)),
	onInstalled: onSearchResultInstalled,
	serverPings,
	getServerModpackContent,
	onContextMenu: (event, result) => {
		if (!isCurseForge(result.project_id)) return handleRightClick(event, result)
		const url =
			curseforgeProjects.get(result.project_id)?.links.websiteUrl ??
			`https://www.curseforge.com/minecraft/mc-mods/${result.slug}`
		contextMenuRef.value?.open(event, [
			{
				id: 'open_link',
				label: '在 CurseForge 打开',
				icon: GlobeIcon,
				action: () => void import('@tauri-apps/plugin-opener').then(({ openUrl }) => openUrl(url)),
			},
			{
				id: 'copy_link',
				label: formatMessage(commonMessages.copyLinkButton),
				action: () => void navigator.clipboard.writeText(url),
			},
		])
	},
	offline,
	lockedFilterMessages,
})
</script>

<template>
	<div class="flex flex-col gap-2 p-6">
		<BrowsePageLayout>
			<template #source-controls>
				<div
					v-if="projectType === 'mod'"
					class="flex flex-wrap items-center gap-2 py-2"
					aria-label="搜索来源"
				>
					<Button
						v-for="source in [
							['all', '全部来源'],
							['modrinth', 'Modrinth'],
							['curseforge', 'CurseForge'],
						] as const"
						:key="source[0]"
						:type="browseSource === source[0] ? 'colored' : 'outlined'"
						:color="browseSource === source[0] ? 'brand' : undefined"
						:aria-pressed="browseSource === source[0]"
						@click="browseSource = source[0]"
						>{{ source[1] }}</Button
					>
				</div>
			</template>
			<template #after>
				<div
					v-for="error in sourceErrors"
					:key="error"
					role="alert"
					class="flex items-center gap-2"
				>
					<span class="text-red">{{ error }}</span
					><Button @click="retrySources">重试</Button>
				</div>
				<p v-if="chineseSearchNote" role="status" class="text-sm text-secondary">
					{{ chineseSearchNote }}
				</p>
				<ContextMenu ref="contextMenuRef" :label="formatMessage(messages.projectActionsLabel)">
					<template #open_link="{ option }">
						<GlobeIcon /> {{ option.label }} <ExternalIcon />
					</template>
				</ContextMenu>
			</template>
		</BrowsePageLayout>
		<CreationFlowModal
			v-if="isServerContext && projectType === 'modpack'"
			ref="serverSetupModalRef"
			:type="serverFlowFrom === 'reset-server' ? 'reset-server' : 'server-onboarding'"
			:available-loaders="['vanilla', 'fabric', 'neoforge', 'forge', 'quilt', 'paper', 'purpur']"
			:show-snapshot-toggle="true"
			:on-back="onServerFlowBack"
			:search-modpacks="searchServerModpacks"
			:get-project-versions="getServerProjectVersions"
			:get-loader-manifest="getLoaderManifest"
			@hide="() => {}"
			@browse-modpacks="() => {}"
			@create="handleServerModpackFlowCreate"
		/>
		<Teleport v-if="browseRouteActive" to="#sidebar-teleport-target">
			<BrowseSidebar />
		</Teleport>
	</div>
</template>
