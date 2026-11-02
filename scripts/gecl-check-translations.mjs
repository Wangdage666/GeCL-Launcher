import fs from 'node:fs'
import { parse } from '@formatjs/icu-messageformat-parser'

function parameters(nodes, result = new Set()) {
	for (const node of nodes) {
		if ([1, 2, 3, 4, 5, 6, 8].includes(node.type)) {
			result.add(`${node.type === 8 ? 'tag' : 'argument'}:${node.value}`)
		}
		if (node.children) parameters(node.children, result)
		for (const option of Object.values(node.options ?? {})) parameters(option.value, result)
	}
	return [...result].sort().join('|')
}

let errors = 0
for (const scope of ['apps/app-frontend', 'packages/ui']) {
	const contract = JSON.parse(fs.readFileSync('scripts/gecl-message-contract.json', 'utf8'))[scope]
	const chinese = JSON.parse(fs.readFileSync(`${scope}/src/locales/zh-CN/index.json`, 'utf8'))
	for (const [key, entry] of Object.entries(contract)) {
		const original = entry
		const translated = chinese[key]?.message ?? chinese[key]?.defaultMessage
		try {
			if (!translated || original !== parameters(parse(translated))) {
				console.error(scope, key, JSON.stringify({ original, translated }))
				errors++
			}
		} catch (error) {
			console.error(scope, key, error.message, JSON.stringify(translated))
			errors++
		}
	}
	console.log(`${scope}: ${Object.keys(contract).length} messages checked`)
}
if (errors) process.exitCode = 1
