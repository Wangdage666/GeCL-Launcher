const compact = (value: string) =>
	value
		.normalize('NFKC')
		.toLowerCase()
		.replace(/[\s\p{P}\p{S}]+/gu, '')
const withoutTrailingTag = (value: string) =>
	value.replace(/\s*[(（[【][^()（）[\]【】]*[)）\]】]\s*$/u, '')

/** 0: a name equals the query, 1: starts with it, 2: contains it, 3: no name match. */
export function nameMatchTier(query: string, names: (string | null | undefined)[]): number {
	const target = compact(query)
	if (!target) return 3
	let tier = 3
	for (const name of names) {
		if (!name) continue
		for (const candidate of [compact(name), compact(withoutTrailingTag(name))]) {
			if (!candidate) continue
			if (candidate === target) return 0
			if (candidate.startsWith(target)) tier = Math.min(tier, 1)
			else if (candidate.includes(target)) tier = Math.min(tier, 2)
		}
	}
	return tier
}
