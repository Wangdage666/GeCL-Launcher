import { Channel, invoke } from '@tauri-apps/api/core'
export interface Author {
	id: number
	name: string
	url: string
	avatarUrl?: string | null
}
export interface Asset {
	title: string
	description: string
	url: string
	thumbnailUrl?: string | null
}
export interface Project {
	id: number
	name: string
	slug: string
	summary: string
	downloadCount: number
	authors: Author[]
	logo?: Asset | null
	screenshots: Asset[]
	categories: { id: number; name: string; slug: string }[]
	links: {
		websiteUrl: string
		sourceUrl?: string | null
		wikiUrl?: string | null
		issuesUrl?: string | null
	}
	dateModified: string
	dateCreated: string
	allowModDistribution?: boolean | null
}
export interface File {
	id: number
	modId: number
	fileName: string
	displayName: string
	fileLength: number
	fileDate: string
	downloadCount: number
	gameVersions: string[]
	releaseType: number
	isAvailable: boolean
	dependencies: { modId: number; relationType: number }[]
}
export interface Page<T> {
	data: T[]
	pagination: { index: number; pageSize: number; resultCount: number; totalCount: number }
}
export interface Search {
	query?: string
	gameVersion?: string
	loader?: string
	index?: number
	pageSize?: number
	sort?: string
	category?: number
	slug?: string
}
export interface InstallProgress {
	phase: string
	name: string
	completed: number
	total: number
}
export const search = (request: Search) =>
	invoke<Page<Project>>('plugin:gecl|curseforge_search', { request })
export const getProject = (id: number) => invoke<Project>('plugin:gecl|curseforge_project', { id })
export const getFiles = (id: number, request: Search) =>
	invoke<Page<File>>('plugin:gecl|curseforge_files', { id, request })
export const getFile = (id: number, fileId: number) =>
	invoke<File>('plugin:gecl|curseforge_file', { id, fileId })
export const getChangelog = (id: number, fileId: number) =>
	invoke<string>('plugin:gecl|curseforge_changelog', { id, fileId })
export const getDescription = (id: number) =>
	invoke<string>('plugin:gecl|curseforge_description', { id })
export const cancelInstall = (jobId: string) =>
	invoke<void>('plugin:gecl|curseforge_cancel', { jobId })
export function install(
	jobId: string,
	instanceId: string,
	projectId: number,
	fileId: number | undefined,
	progress: (value: InstallProgress) => void,
) {
	const onProgress = new Channel<InstallProgress>()
	onProgress.onmessage = progress
	return invoke<string[]>('plugin:gecl|curseforge_install', {
		jobId,
		instanceId,
		projectId,
		fileId,
		onProgress,
	})
}
export const message = (error: unknown) => (error as { message?: string })?.message ?? String(error)
export const mcmodProject = (project: Project) => ({
	platform: 'curseforge' as const,
	id: String(project.id),
	slug: project.slug,
	title: project.name,
})

export const getInstalledProjects = () =>
	invoke<Record<string, number[]>>('plugin:gecl|curseforge_installed')
