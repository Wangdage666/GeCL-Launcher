import type { ExternalContentInstall } from '@/providers/content-install'
import { getInstanceIconUrl, list } from '@/helpers/instance'
import { getFiles, getProject, getInstalledProjects, type File } from './api'
import { projectKey } from './search'
import { startDownload, waitForDownload } from './tasks'

const loaders = ['forge', 'neoforge', 'fabric', 'quilt']
function matches(file: File, game: { game_version: string; loader: string }) {
	return (
		file.isAvailable &&
		file.gameVersions.includes(game.game_version) &&
		file.gameVersions.some((value) => value.toLowerCase() === game.loader)
	)
}
async function matchingFile(
	projectId: number,
	game: { game_version: string; loader: string },
	pinned?: number,
) {
	for (let index = 0; index < 1000; index += 50) {
		const page = await getFiles(projectId, {
			gameVersion: game.game_version,
			loader: game.loader,
			pageSize: 50,
			index,
		})
		const file = page.data
			.filter((file) => matches(file, game) && (!pinned || file.id === pinned))
			.sort((a, b) => b.fileDate.localeCompare(a.fileDate))[0]
		if (file) return file
		if (!page.data.length || index + page.data.length >= page.pagination.totalCount) break
	}
	return undefined
}
export async function createCurseForgeInstall(
	projectId: number,
	pinned?: number,
	loadChoices = true,
): Promise<ExternalContentInstall> {
	const project = await getProject(projectId)
	const initial = loadChoices ? await getFiles(projectId, { pageSize: 50 }) : { data: [] }
	const versions = [
		...new Set(
			initial.data.flatMap((file) =>
				file.gameVersions.filter(
					(value) =>
						/^(?:\d|a\d|b\d|rd-|inf-)/i.test(value) &&
						!loaders.includes(value.toLowerCase()) &&
						!['client', 'server'].includes(value.toLowerCase()),
				),
			),
		),
	]
	const supportedLoaders = loaders.filter((loader) =>
		initial.data.some((file) => file.gameVersions.some((value) => value.toLowerCase() === loader)),
	)
	return {
		projectInfo: {
			title: project.name,
			iconUrl: project.logo?.thumbnailUrl ?? project.logo?.url,
			link: `/curseforge/${projectId}`,
		},
		loaders: supportedLoaders,
		gameVersions: versions,
		async candidates() {
			const [allGames, installed] = await Promise.all([list(), getInstalledProjects()])
			const games = allGames.filter((game) => !game.quarantined)
			const targets = [
				...new Map(games.map((game) => [`${game.game_version}:${game.loader}`, game])).entries(),
			]
			const compatible = new Map<string, boolean>()
			for (let index = 0; index < targets.length; index += 4) {
				await Promise.all(
					targets.slice(index, index + 4).map(async ([key, game]) => {
						compatible.set(
							key,
							loaders.includes(game.loader) && !!(await matchingFile(projectId, game, pinned)),
						)
					}),
				)
			}
			return games.map((game) => ({
				id: game.id,
				name: game.name,
				iconUrl: getInstanceIconUrl(game.icon_path),
				compatible: compatible.get(`${game.game_version}:${game.loader}`) ?? false,
				installed: installed[game.id]?.includes(projectId) ?? false,
				installing: false,
			}))
		},
		async install(game) {
			const file = await matchingFile(projectId, game, pinned)
			if (!file) throw new Error(`没有适用于 ${game.game_version} / ${game.loader} 的模组版本。`)
			const task = startDownload({
				instanceId: game.id,
				projectId,
				fileId: file.id,
				title: project.name,
				icon: project.logo?.thumbnailUrl ?? project.logo?.url,
			})
			await waitForDownload(task)
			const installed = await getInstalledProjects()
			return {
				versionId: String(file.id),
				projectIds: [
					...new Set([projectKey(projectId), ...(installed[game.id] ?? []).map(projectKey)]),
				],
			}
		},
	}
}
