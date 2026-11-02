import cssText from '@modrinth/assets/styles/ace.css?raw'
import ace from 'ace-builds'

/**
 * Registers only the Ace modules reachable from `getEditorLanguage` and the default editor
 * commands, instead of `ace-builds/esm-resolver`, which bundles every mode, theme, snippet and
 * worker. Ace creates workers from URLs, so worker loaders were never used here.
 */
const aceModuleLoaders: Record<string, () => Promise<unknown>> = {
	'ace/ext/prompt': () => import('ace-builds/src-noconflict/ext-prompt'),
	'ace/ext/searchbox': () => import('ace-builds/src-noconflict/ext-searchbox'),
	'ace/ext/settings_menu': () => import('ace-builds/src-noconflict/ext-settings_menu'),
	'ace/mode/batchfile': () => import('ace-builds/src-noconflict/mode-batchfile'),
	'ace/mode/c_cpp': () => import('ace-builds/src-noconflict/mode-c_cpp'),
	'ace/mode/css': () => import('ace-builds/src-noconflict/mode-css'),
	'ace/mode/golang': () => import('ace-builds/src-noconflict/mode-golang'),
	'ace/mode/html': () => import('ace-builds/src-noconflict/mode-html'),
	'ace/mode/ini': () => import('ace-builds/src-noconflict/mode-ini'),
	'ace/mode/java': () => import('ace-builds/src-noconflict/mode-java'),
	'ace/mode/javascript': () => import('ace-builds/src-noconflict/mode-javascript'),
	'ace/mode/json': () => import('ace-builds/src-noconflict/mode-json'),
	'ace/mode/markdown': () => import('ace-builds/src-noconflict/mode-markdown'),
	'ace/mode/php': () => import('ace-builds/src-noconflict/mode-php'),
	'ace/mode/powershell': () => import('ace-builds/src-noconflict/mode-powershell'),
	'ace/mode/properties': () => import('ace-builds/src-noconflict/mode-properties'),
	'ace/mode/python': () => import('ace-builds/src-noconflict/mode-python'),
	'ace/mode/ruby': () => import('ace-builds/src-noconflict/mode-ruby'),
	'ace/mode/rust': () => import('ace-builds/src-noconflict/mode-rust'),
	'ace/mode/sh': () => import('ace-builds/src-noconflict/mode-sh'),
	'ace/mode/text': () => import('ace-builds/src-noconflict/mode-text'),
	'ace/mode/toml': () => import('ace-builds/src-noconflict/mode-toml'),
	'ace/mode/typescript': () => import('ace-builds/src-noconflict/mode-typescript'),
	'ace/mode/yaml': () => import('ace-builds/src-noconflict/mode-yaml'),
}

for (const [moduleName, load] of Object.entries(aceModuleLoaders)) {
	ace.config.setModuleLoader(moduleName, load)
}

ace['define'](
	'ace/theme/modrinth',
	['require', 'exports', 'module', 'ace/lib/dom'],
	function (require, exports, _module) {
		exports.isDark = false
		exports.cssClass = 'ace-modrinth'
		exports.cssText = cssText

		const dom = require('ace/lib/dom')
		dom.importCssString(exports.cssText, exports.cssClass, false)
	},
)
