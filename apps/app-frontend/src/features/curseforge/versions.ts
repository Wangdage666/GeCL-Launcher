import type { Labrinth } from '@modrinth/api-client'
import { getFile, getFiles, type File, type Search } from './api'

export const fileKey = (id: number | string) => `curseforge:${id}`
const loaders = new Set(['forge', 'neoforge', 'fabric', 'quilt'])

export function toVersion(file: File): Labrinth.Versions.v2.Version {
	return {
		id: fileKey(file.id),
		project_id: fileKey(file.modId),
		author_id: '',
		name: file.displayName,
		version_number: file.displayName,
		date_published: file.fileDate,
		downloads: file.downloadCount,
		game_versions: file.gameVersions.filter((value) => /^(?:\d|a\d|b\d|rd-|inf-)/i.test(value)),
		loaders: file.gameVersions
			.map((value) => value.toLowerCase())
			.filter((value) => loaders.has(value)),
		version_type: file.releaseType === 2 ? 'beta' : file.releaseType === 3 ? 'alpha' : 'release',
		featured: false,
		status: file.isAvailable ? 'listed' : 'archived',
		dependencies: [],
		files: [],
	}
}

export async function loadVersions(
	projectId: number,
	currentFileId: number,
	request: Search,
	active: () => boolean,
	publish: (versions: Labrinth.Versions.v2.Version[]) => void,
) {
	const current = await getFile(projectId, currentFileId)
	if (!active()) return
	publish([toVersion(current)])
	for (let index = 0; index < 10000; index += 50) {
		const page = await getFiles(projectId, { ...request, index, pageSize: 50 })
		if (!active()) return
		publish(page.data.map(toVersion))
		if (!page.data.length || index + page.data.length >= page.pagination.totalCount) return
	}
}
