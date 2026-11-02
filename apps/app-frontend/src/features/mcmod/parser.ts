export interface McmodProject {
	platform?: 'curseforge'
	id?: string
	project_id?: string
	slug?: string | null
	title: string
}
export interface McmodEntry {
	classId: number
	name: string
	originalName: string
}
const canonicalName = (name: string) => name.normalize('NFKC').trim().toLowerCase()
export function lookupQueries(project: McmodProject): string[] {
	if (project.platform !== 'curseforge') return [project.title.trim()]
	const title = project.title.replace(/^(?:\[[^\]]+\]\s*)+/, '').trim()
	return [
		...new Set(
			[
				title.replace(/\s*[（(][^()（）]+[）)]\s*$/, '').trim(),
				project.slug?.replace(/[-_]+/g, ' ').trim(),
				project.title.trim(),
			].filter((value): value is string => !!value),
		),
	]
}
export function searchCandidates(html: string, title: string, limit = 4): number[] {
	const document = new DOMParser().parseFromString(html, 'text/html')
	const candidates = new Map<number, number>()
	for (const anchor of document.querySelectorAll<HTMLAnchorElement>('.head a[href]')) {
		let url: URL
		try {
			url = new URL(anchor.getAttribute('href') ?? '', 'https://www.mcmod.cn')
		} catch {
			continue
		}
		const match = /^\/class\/(\d+)\.html$/.exec(url.pathname)
		if (url.hostname !== 'www.mcmod.cn' || !match) continue
		const text = (anchor.textContent ?? '').trim()
		const english = /[（(]([^()（）]+)[）)]$/.exec(text)?.[1] ?? text.replace(/^\[[^\]]+\]\s*/, '')
		const chinese = text
			.replace(/^\[[^\]]+\]\s*/, '')
			.replace(/[（(][^()（）]+[）)]$/, '')
			.trim()
		const exact =
			canonicalName(english) === canonicalName(title) ||
			canonicalName(chinese) === canonicalName(title)
		candidates.set(Number(match[1]), exact ? 0 : 1)
	}
	return [...candidates]
		.sort((a, b) => a[1] - b[1])
		.slice(0, limit)
		.map(([id]) => id)
}
function targetUrl(href: string): URL | null {
	try {
		let url = new URL(href, 'https://www.mcmod.cn')
		if (url.hostname === 'link.mcmod.cn' && url.pathname.startsWith('/target/')) {
			const encoded = url.pathname.slice('/target/'.length).replace(/-/g, '+').replace(/_/g, '/')
			url = new URL(atob(encoded))
		}
		return url
	} catch {
		return null
	}
}
export interface LinkedMcmodEntry extends McmodEntry {
	identifiers: string[]
	curseforgeSlugs?: string[]
	curseforgeIds?: string[]
}
export function linkedEntry(html: string, classId: number): LinkedMcmodEntry | null {
	if (!Number.isSafeInteger(classId) || classId <= 0) return null
	const document = new DOMParser().parseFromString(html, 'text/html')
	// Only the entry's related-links section counts, never article text or recommendations.
	const links = document.querySelector('.class-info-left .common-link-frame')
	if (!links) return null
	const identifiers = new Set<string>()
	const curseforgeSlugs = new Set<string>()
	const curseforgeIds = new Set<string>()
	for (const anchor of links.querySelectorAll<HTMLAnchorElement>('a[href]')) {
		const url = targetUrl(anchor.getAttribute('href') ?? '')
		if (!url || !['http:', 'https:'].includes(url.protocol)) continue
		if (['modrinth.com', 'www.modrinth.com'].includes(url.hostname)) {
			const match = /^\/(?:mod|project)\/([A-Za-z0-9_-]{1,128})\/?$/.exec(url.pathname)
			if (match) identifiers.add(match[1])
		} else if (
			[
				'curseforge.com',
				'www.curseforge.com',
				'legacy.curseforge.com',
				'minecraft.curseforge.com',
			].includes(url.hostname)
		) {
			const match = /^(?:\/minecraft\/mc-mods|\/projects)\/([A-Za-z0-9_-]{1,128})\/?$/.exec(
				url.pathname,
			)
			if (match) {
				if (/^[1-9]\d*$/.test(match[1])) curseforgeIds.add(match[1])
				else curseforgeSlugs.add(match[1].toLowerCase())
			}
		}
	}
	if (!identifiers.size && !curseforgeSlugs.size && !curseforgeIds.size) return null
	const name = document.querySelector('.class-title h3')?.textContent?.trim() ?? ''
	const originalName = document.querySelector('.class-title h4')?.textContent?.trim() ?? ''
	if (!name || name.length > 120) return null
	return {
		classId,
		name,
		originalName,
		identifiers: [...identifiers],
		curseforgeSlugs: [...curseforgeSlugs],
		curseforgeIds: [...curseforgeIds],
	}
}
export function verifiedEntry(
	html: string,
	classId: number,
	project: McmodProject,
): McmodEntry | null {
	const entry = linkedEntry(html, classId)
	if (
		!entry ||
		!(project.platform === 'curseforge'
			? (project.slug && entry.curseforgeSlugs?.includes(project.slug.toLowerCase())) ||
				(project.id && entry.curseforgeIds?.includes(project.id))
			: [project.id, project.project_id, project.slug].some(
					(id) => id && entry.identifiers.includes(id),
				))
	)
		return null
	return { classId, name: entry.name, originalName: entry.originalName || project.title }
}
export function displayModName(original: string, entry: McmodEntry | null): string {
	if (!entry || !/\p{Script=Han}/u.test(entry.name) || entry.name === original) return original
	return `${entry.name}（${original}）`
}
export function mcmodUrl(entry: McmodEntry | null, original: string): string {
	if (entry) return `https://www.mcmod.cn/class/${entry.classId}.html`
	const url = new URL('https://search.mcmod.cn/s')
	url.searchParams.set('key', original)
	url.searchParams.set('mold', '1')
	return url.toString()
}
