import { fetch } from '@tauri-apps/plugin-http'
import {
	displayModName,
	mcmodUrl,
	lookupQueries,
	searchCandidates,
	verifiedEntry,
	type McmodEntry,
	type McmodProject,
} from './parser'
export { displayModName, mcmodUrl }
export type { McmodEntry, McmodProject }
type Cached = { slug?: string | null; entry: McmodEntry | null; expires: number }
const cache = new Map<string, Cached>()
type LookupResult = { entry: McmodEntry | null; failed: boolean }
const pending = new Map<string, Promise<LookupResult>>()
const STORAGE_KEY = 'gecl-mcmod-v2'
try {
	const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')
	for (const [key, value] of Object.entries(stored) as [string, Cached][]) {
		if (!value || !value.entry || !Number.isFinite(value.expires) || value.expires <= Date.now())
			continue
		const entry = value.entry
		if (
			entry &&
			(!Number.isSafeInteger(entry.classId) ||
				entry.classId <= 0 ||
				typeof entry.name !== 'string' ||
				entry.name.length > 120)
		)
			continue
		cache.set(key, value)
	}
} catch {
	/* A malformed optional cache never prevents resource browsing. */
}
const keyFor = (project: McmodProject) => {
	const id = project.project_id ?? project.id ?? project.slug
	return id ? `${project.platform ?? 'modrinth'}:${id}` : ''
}
export function cachedMcmodEntry(project: McmodProject): McmodEntry | null {
	const cached = cache.get(keyFor(project))
	return cached && cached.expires > Date.now() && cached.slug === project.slug ? cached.entry : null
}
let requestQueue = Promise.resolve()
export async function publicHtml(url: string, signal?: AbortSignal): Promise<string> {
	// At most two public page requests per second, shared by search and detail pages.
	const previous = requestQueue
	requestQueue = previous.then(() => new Promise((resolve) => setTimeout(resolve, 500)))
	await previous
	signal?.throwIfAborted()
	const controller = new AbortController()
	const abort = () => controller.abort(signal?.reason)
	signal?.addEventListener('abort', abort, { once: true })
	const timer = setTimeout(() => controller.abort(), 10_000)
	try {
		const response = await fetch(url, {
			method: 'GET',
			signal: controller.signal,
			connectTimeout: 5_000,
		})
		if (!response.ok) throw new Error(`MC百科 HTTP ${response.status}`)
		const text = await response.text()
		if (text.length > 2_000_000) throw new Error('MC百科页面过大')
		return text
	} finally {
		clearTimeout(timer)
		signal?.removeEventListener('abort', abort)
	}
}
export function rememberMcmod(project: McmodProject, entry: McmodEntry) {
	cache.set(keyFor(project), { slug: project.slug, entry, expires: Date.now() + 7 * 86400_000 })
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(Object.fromEntries(cache)))
	} catch {
		/* Optional cache. */
	}
}
export async function resolveMcmod(
	project: McmodProject,
	signal?: AbortSignal,
	options: { retryMissing?: boolean; throwOnError?: boolean } = {},
): Promise<McmodEntry | null> {
	const key = keyFor(project)
	if (!key || typeof project.title !== 'string' || !project.title.trim()) return null
	const cached = cache.get(key)
	if (
		cached &&
		cached.expires > Date.now() &&
		cached.slug === project.slug &&
		(cached.entry || !options.retryMissing)
	)
		return cached.entry
	let work = pending.get(key)
	if (!work) {
		work = (async (): Promise<LookupResult> => {
			const controller = new AbortController()
			const abort = () => controller.abort()
			signal?.addEventListener('abort', abort, { once: true })
			if (signal?.aborted) abort()
			const timer = setTimeout(abort, 20_000)
			let entry: McmodEntry | null = null
			let failed = false
			const visited = new Set<number>()
			try {
				for (const title of lookupQueries(project)) {
					if (controller.signal.aborted) {
						failed = true
						break
					}
					try {
						const query = new URL('https://search.mcmod.cn/s')
						query.searchParams.set('key', title)
						query.searchParams.set('mold', '1')
						const html = await publicHtml(query.toString(), controller.signal)
						for (const classId of searchCandidates(html, title)) {
							if (controller.signal.aborted) {
								failed = true
								break
							}
							if (visited.has(classId)) continue
							visited.add(classId)
							try {
								const detail = await publicHtml(
									`https://www.mcmod.cn/class/${classId}.html`,
									controller.signal,
								)
								entry = verifiedEntry(detail, classId, project)
								if (entry) break
							} catch {
								failed = true
							}
						}
					} catch {
						failed = true
					}
					if (entry) break
				}
				if (controller.signal.aborted) return { entry: null, failed: true }
				if (entry || !failed) {
					cache.set(key, {
						slug: project.slug,
						entry,
						expires: Date.now() + (entry ? 7 * 86400_000 : 600_000),
					})
					while (cache.size > 128) cache.delete(cache.keys().next().value!)
					try {
						localStorage.setItem(STORAGE_KEY, JSON.stringify(Object.fromEntries(cache)))
					} catch {
						/* Optional cache. */
					}
				}
				return { entry, failed }
			} finally {
				clearTimeout(timer)
				signal?.removeEventListener('abort', abort)
			}
		})()
		pending.set(key, work)
	}
	try {
		const result = await work
		if (signal?.aborted) return null
		if (!result.entry && result.failed && options.throwOnError)
			throw new Error('暂时无法查询 MC 百科，请重试。')
		return result.entry
	} finally {
		if (pending.get(key) === work) pending.delete(key)
	}
}
