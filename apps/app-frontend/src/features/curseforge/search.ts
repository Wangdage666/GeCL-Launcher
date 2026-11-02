import type { Labrinth } from '@modrinth/api-client'
import { cachedMcmodEntry, displayModName, rememberMcmod } from '@/features/mcmod'
import type { LinkedMcmodEntry } from '@/features/mcmod/parser'
import { mcmodProject, search, type Project, type Search } from './api'
export const projects = new Map<string, Project>()
export const projectKey = (id: number) => `curseforge:${id}`
export const isCurseForge = (id: string) => id.startsWith('curseforge:')
export function toSearchResult(project: Project): Labrinth.Search.v3.ResultSearchProject {
	const id = projectKey(project.id)
	projects.set(id, project)
	return {
		project_id: id,
		project_types: ['mod'],
		all_project_types: ['mod'],
		slug: project.slug,
		name: displayModName(project.name, cachedMcmodEntry(mcmodProject(project))),
		summary: project.summary,
		author: project.authors.map((author) => author.name).join('、'),
		author_id: null,
		organization: null,
		organization_id: null,
		categories: [],
		display_categories: [],
		downloads: project.downloadCount,
		follows: 0,
		icon_url: project.logo?.thumbnailUrl ?? project.logo?.url ?? null,
		date_created: project.dateCreated,
		date_modified: project.dateModified,
		license: '',
		gallery: project.screenshots.map((image) => image.url),
		featured_gallery: project.screenshots[0]?.url ?? null,
		color: null,
		loaders: [],
		disclosure_types: [],
	}
}
export async function searchProjects(request: Search, entries: LinkedMcmodEntry[]) {
	const slugs = [...new Set(entries.flatMap((entry) => entry.curseforgeSlugs ?? []))]
	if (/\p{Script=Han}/u.test(request.query ?? '') && slugs.length) {
		const responses = await Promise.all(
			slugs.map((slug) => search({ ...request, query: '', slug, index: 0, pageSize: 50 })),
		)
		const matches = [
			...new Map(
				responses.flatMap((response) => response.data).map((project) => [project.id, project]),
			).values(),
		]
		for (const project of matches) {
			const entry = entries.find((entry) => entry.curseforgeSlugs?.includes(project.slug))
			if (entry) rememberMcmod(mcmodProject(project), entry)
		}
		const start = request.index ?? 0
		return {
			items: matches.slice(start, start + (request.pageSize ?? 50)).map(toSearchResult),
			total: matches.length,
		}
	}
	const response = await search(request)
	return { items: response.data.map(toSearchResult), total: response.pagination.totalCount }
}
