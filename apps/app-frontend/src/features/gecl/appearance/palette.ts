export type VisualPreferences = {
	accentColor: string
	backgroundId: string | null
	backgroundEnabled: boolean
	blur: number
	overlay: number
	surfaceOpacity: number
	position: 'center' | 'top' | 'bottom'
}

export const DEFAULT_VISUAL_PREFERENCES: VisualPreferences = {
	accentColor: '#176fb0',
	backgroundId: null,
	backgroundEnabled: true,
	blur: 8,
	overlay: 18,
	surfaceOpacity: 88,
	position: 'center',
}

export const ACCENT_PRESETS = [
	{ name: '海蓝', color: '#176fb0' },
	{ name: '靛青', color: '#5956ad' },
	{ name: '青绿', color: '#087d83' },
	{ name: '森林', color: '#367849' },
	{ name: '琥珀', color: '#a96716' },
	{ name: '珊瑚', color: '#b74d3e' },
	{ name: '玫红', color: '#a74374' },
	{ name: '石墨', color: '#536273' },
] as const

type RGB = [number, number, number]

export function validAccent(value: string): boolean {
	return /^#[\da-f]{6}$/i.test(value)
}

function rgb(hex: string): RGB {
	return [1, 3, 5].map((start) => Number.parseInt(hex.slice(start, start + 2), 16)) as RGB
}

export function luminance(color: RGB): number {
	const channels = color.map((value) => {
		const s = value / 255
		return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
	})
	return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722
}

export function contrast(first: RGB, second: RGB): number {
	const a = luminance(first)
	const b = luminance(second)
	return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)
}

export function accentTokens(
	hex: string,
	dark: boolean,
	overPhoto = false,
): Record<string, string> {
	const selected = rgb(validAccent(hex) ? hex : DEFAULT_VISUAL_PREFERENCES.accentColor)
	const against: RGB = dark
		? overPhoto
			? [69, 70, 73]
			: [52, 54, 60]
		: overPhoto
			? [204, 204, 204]
			: [255, 255, 255]
	const target = dark ? 255 : 0
	let result = selected
	if (contrast(selected, against) < 4.6) {
		let low = 0
		let high = 1
		for (let i = 0; i < 16; i++) {
			const factor = (low + high) / 2
			const candidate = selected.map((v) => Math.round(v + (target - v) * factor)) as RGB
			if (contrast(candidate, against) < 4.6) low = factor
			else high = factor
		}
		result = selected.map((v) => Math.round(v + (target - v) * high)) as RGB
	}
	const color = `rgb(${result.join(', ')})`
	const highlight = `rgba(${result.join(', ')}, ${dark ? 0.2 : 0.14})`
	return {
		'--color-brand': color,
		'--color-brand-highlight': highlight,
		'--color-brand-shadow': `rgba(${result.join(', ')}, 0.32)`,
		'--color-button-bg-selected': dark ? highlight : color,
		'--color-button-text-selected': dark ? color : '#ffffff',
		'--loading-bar-gradient': `linear-gradient(to right, ${color}, ${color})`,
	}
}

export function backdropTokens(value: VisualPreferences, dark: boolean): Record<string, string> {
	const base = dark ? '22, 24, 28' : '255, 255, 255'
	const opacity = Math.max(80, Math.min(100, value.surfaceOpacity)) / 100
	return {
		'--gecl-background-blur': `${value.blur}px`,
		'--gecl-background-position': `center ${value.position}`,
		'--gecl-background-overlay': `rgba(${base}, ${value.overlay / 100})`,
		'--gecl-content-surface': `rgba(${base}, ${opacity})`,
		'--gecl-shell-surface': `rgba(${dark ? '29, 31, 35' : '248, 249, 251'}, ${Math.max(0.84, opacity)})`,
	}
}
