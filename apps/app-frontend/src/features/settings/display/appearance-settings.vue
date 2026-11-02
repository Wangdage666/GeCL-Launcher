<script setup lang="ts">
import { CheckIcon, ImageIcon, RotateCounterClockwiseIcon, XIcon } from '@modrinth/assets'
import { Button, Toggle, useSavable } from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { platform } from '@tauri-apps/plugin-os'
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'

import { useAppSettings } from '@/composables/use-app-settings'
import { type ColorTheme, isDarkTheme, useTheme } from '@/composables/use-theme'
import {
	ACCENT_PRESETS,
	DEFAULT_VISUAL_PREFERENCES,
	validAccent,
	type VisualPreferences,
} from '@/features/gecl/appearance/palette'
import {
	type Background,
	discardBackground,
	importBackground,
	visualAppearance,
} from '@/features/gecl/appearance/state'
import { injectAppSettingsModal } from '@/features/settings/app-settings-modal/context'

const theme = useTheme()
const appSettings = useAppSettings()
const settingsModal = injectAppSettingsModal()
const error = ref('')
const importing = ref(false)
const ready = ref(false)
const images = reactive(new Map<string, Background>())
const cleanupIds = new Set<string>()
const os = platform()
let disposed = false

type AppearanceState = {
	theme: ColorTheme
	advancedRendering: boolean
	nativeDecorations: boolean
	visual: VisualPreferences
}

function original(): AppearanceState {
	return {
		theme: theme.preferred,
		advancedRendering: theme.advancedRendering,
		nativeDecorations: appSettings.nativeDecorations,
		visual: { ...visualAppearance.saved.value.preferences },
	}
}

const { saved, current, changes, saving, hasChanges, reset, save } = useSavable(
	original,
	async () => {
		const value = current.value
		await visualAppearance.persist(value.visual, {
			theme: value.theme,
			advancedRendering: value.advancedRendering,
			nativeDecorations: value.nativeDecorations,
		})
		if (isDarkTheme(value.theme)) theme.preferredDark = value.theme
		theme.preferred = value.theme
		theme.advancedRendering = value.advancedRendering
		appSettings.nativeDecorations = value.nativeDecorations
	},
)

const modes: { value: ColorTheme; label: string }[] = [
	{ value: 'light', label: '浅色' },
	{ value: 'dark', label: '深色' },
	{ value: 'oled', label: '纯黑' },
	{ value: 'system', label: '跟随系统' },
]
if (theme.preferred === 'retro') modes.push({ value: 'retro', label: '复古' })

const selectedBackground = computed(() => {
	const id = current.value.visual.backgroundId
	if (!id) return null
	return (
		images.get(id) ??
		(visualAppearance.saved.value.background?.id === id
			? visualAppearance.saved.value.background
			: null)
	)
})
const photoStyle = computed(() =>
	selectedBackground.value
		? {
				backgroundImage:
					'url(' + JSON.stringify(convertFileSrc(selectedBackground.value.path)) + ')',
			}
		: {},
)

watch(
	[current, selectedBackground],
	() => {
		if (!ready.value) return
		theme.preview = current.value.theme === saved.value.theme ? null : current.value.theme
		visualAppearance.preview.value = {
			preferences: { ...current.value.visual },
			background: selectedBackground.value,
		}
	},
	{ deep: true },
)

function errorMessage(value: unknown, fallback: string): string {
	return value instanceof Error ? value.message : typeof value === 'string' ? value : fallback
}

async function load(): Promise<void> {
	error.value = ''
	try {
		await visualAppearance.load()
		if (disposed) return
		reset()
		ready.value = true
	} catch {
		error.value = '无法读取外观设置，请重试'
	}
}

async function chooseBackground(): Promise<void> {
	error.value = ''
	importing.value = true
	try {
		const source = await open({
			multiple: false,
			directory: false,
			filters: [{ name: '图片', extensions: ['jpg', 'jpeg', 'png', 'webp'] }],
		})
		if (typeof source !== 'string') return
		const background = await importBackground(source)
		cleanupIds.add(background.id)
		if (disposed) {
			void discardBackground(background.id).catch(() => {})
			return
		}
		images.set(background.id, background)
		current.value.visual.backgroundId = background.id
		current.value.visual.backgroundEnabled = true
	} catch (value) {
		error.value = errorMessage(value, '无法导入图片，请更换图片')
	} finally {
		importing.value = false
	}
}

function updateCustomColor(event: Event): void {
	const input = event.target as HTMLInputElement
	if (validAccent(input.value)) {
		current.value.visual.accentColor = input.value.toLowerCase()
		error.value = ''
	} else {
		input.value = current.value.visual.accentColor
		error.value = '请输入六位十六进制颜色，例如 #176fb0'
	}
}

function restoreDefaults(): void {
	current.value.theme = 'light'
	current.value.visual = { ...DEFAULT_VISUAL_PREFERENCES }
	error.value = ''
}

async function saveAppearance(): Promise<void> {
	error.value = ''
	const previousId = visualAppearance.saved.value.preferences.backgroundId
	try {
		await save()
		if (previousId) cleanupIds.add(previousId)
	} catch (value) {
		error.value = errorMessage(value, '无法保存外观，请重试')
	}
}

onMounted(() => {
	settingsModal.registerUnsavedChangesController({
		hasChanges: () => hasChanges.value || importing.value,
		getOriginal: () => saved.value,
		getModified: () => changes.value,
		isSaving: () => saving.value || importing.value,
		reset,
		save: saveAppearance,
	})
	void load()
})

onBeforeUnmount(() => {
	disposed = true
	theme.preview = null
	visualAppearance.preview.value = null
	settingsModal.registerUnsavedChangesController(null)
	for (const id of cleanupIds) void discardBackground(id).catch(() => {})
})
</script>

<template>
	<div class="gecl-appearance-settings">
		<div v-if="error" role="alert" class="appearance-error">
			<span>{{ error }}</span
			><button v-if="!ready" class="retry-button" @click="load">重试</button>
		</div>
		<fieldset :disabled="!ready || saving || importing" :aria-busy="!ready || saving || importing">
			<section aria-labelledby="appearance-mode-heading">
				<h2 id="appearance-mode-heading">界面模式</h2>
				<div class="mode-options" role="group" aria-label="界面模式">
					<button
						v-for="mode in modes"
						:key="mode.value"
						:aria-pressed="current.theme === mode.value"
						@click="current.theme = mode.value"
					>
						<span class="mode-dot" :data-theme="mode.value" aria-hidden="true"></span
						>{{ mode.label }}
					</button>
				</div>
			</section>
			<section aria-labelledby="appearance-accent-heading">
				<h2 id="appearance-accent-heading">主题色</h2>
				<div class="accent-controls">
					<div class="accent-swatches" role="group" aria-label="预设主题色">
						<button
							v-for="preset in ACCENT_PRESETS"
							:key="preset.color"
							:aria-label="preset.name"
							:title="preset.name"
							:aria-pressed="current.visual.accentColor === preset.color"
							class="accent-swatch"
							:style="{ '--swatch': preset.color }"
							@click="current.visual.accentColor = preset.color"
						>
							<CheckIcon v-if="current.visual.accentColor === preset.color" aria-hidden="true" />
						</button>
					</div>
					<div class="custom-accent">
						<label for="appearance-color">自定义</label>
						<input id="appearance-color" v-model="current.visual.accentColor" type="color" />
						<input
							class="color-hex"
							:value="current.visual.accentColor"
							aria-label="主题色代码"
							maxlength="7"
							spellcheck="false"
							@change="updateCustomColor"
						/>
					</div>
				</div>
			</section>
			<section
				aria-labelledby="appearance-background-heading"
				:class="{ 'has-background': selectedBackground }"
			>
				<div class="section-title-row">
					<h2 id="appearance-background-heading">背景图片</h2>
					<Toggle
						v-if="selectedBackground"
						v-model="current.visual.backgroundEnabled"
						aria-label="显示背景图片"
					/>
				</div>
				<div
					v-if="selectedBackground"
					class="background-preview"
					:class="{ 'background-disabled': !current.visual.backgroundEnabled }"
					aria-label="背景效果预览"
				>
					<div class="preview-photo" :style="photoStyle"></div>
					<div class="preview-overlay"></div>
					<div class="preview-window">
						<div class="preview-nav" aria-hidden="true">
							<span></span><span class="preview-nav-active"></span><span></span>
						</div>
						<div class="preview-content">
							<strong>游戏版本</strong><span class="preview-detail">Minecraft · Forge</span
							><span class="preview-action">开始游戏</span>
						</div>
					</div>
				</div>
				<div class="background-actions">
					<span v-if="!selectedBackground" class="background-status">{{
						current.visual.backgroundId ? '图片不可用，请重新选择' : '未设置背景'
					}}</span>
					<Button :loading="importing" @click="chooseBackground">
						<ImageIcon />{{
							importing ? '正在处理…' : selectedBackground ? '更换图片' : '选择图片'
						}}
					</Button>
					<Button
						v-if="current.visual.backgroundId"
						type="quiet"
						@click="current.visual.backgroundId = null"
					>
						<XIcon />移除
					</Button>
				</div>
				<div v-if="selectedBackground" class="background-controls">
					<label class="range-control"
						><span
							>背景模糊 <output>{{ current.visual.blur }} px</output></span
						><input
							v-model.number="current.visual.blur"
							type="range"
							min="0"
							max="24"
							aria-label="背景模糊"
					/></label>
					<label class="range-control"
						><span
							>背景遮罩 <output>{{ current.visual.overlay }}%</output></span
						><input
							v-model.number="current.visual.overlay"
							type="range"
							min="0"
							max="60"
							aria-label="背景遮罩"
					/></label>
					<label class="range-control"
						><span
							>面板不透明度 <output>{{ current.visual.surfaceOpacity }}%</output></span
						><input
							v-model.number="current.visual.surfaceOpacity"
							type="range"
							min="80"
							max="100"
							aria-label="面板不透明度"
					/></label>
					<div class="position-control">
						<label for="appearance-position">图片对齐</label
						><select id="appearance-position" v-model="current.visual.position">
							<option value="center">居中</option>
							<option value="top">顶部</option>
							<option value="bottom">底部</option>
						</select>
					</div>
				</div>
			</section>
			<section class="appearance-advanced" aria-label="其他外观选项">
				<div class="toggle-row">
					<label for="appearance-shadows">界面阴影</label
					><Toggle id="appearance-shadows" v-model="current.advancedRendering" />
				</div>
				<div v-if="os !== 'macos'" class="toggle-row">
					<label for="appearance-native" title="重新启动后生效">系统标题栏</label
					><Toggle id="appearance-native" v-model="current.nativeDecorations" />
				</div>
				<Button type="quiet" @click="restoreDefaults">
					<RotateCounterClockwiseIcon />恢复默认外观
				</Button>
			</section>
		</fieldset>
	</div>
</template>

<style scoped>
.gecl-appearance-settings {
	max-width: 900px;
}
fieldset {
	padding: 0;
	margin: 0;
	border: 0;
	min-width: 0;
}
section + section {
	margin-top: 24px;
	padding-top: 22px;
	border-top: 1px solid var(--color-divider);
}
h2 {
	margin: 0 0 14px;
	font-size: 18px;
	font-weight: 650;
	color: var(--color-contrast);
}
button,
select,
input {
	font: inherit;
}
button {
	cursor: pointer;
}
button:disabled,
fieldset:disabled button {
	cursor: default;
	opacity: 0.55;
}
button:focus-visible,
select:focus-visible,
input:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 3px;
}
.mode-options {
	display: flex;
	flex-wrap: wrap;
	gap: 8px;
}
.mode-options button {
	display: flex;
	align-items: center;
	justify-content: center;
	gap: 9px;
	min-height: 40px;
	padding: 7px 16px;
	border: 1px solid var(--color-divider);
	border-radius: 9px;
	color: var(--color-base);
	background: var(--surface-4);
}
.mode-options button[aria-pressed='true'] {
	background: var(--color-brand-highlight);
	color: var(--color-brand);
	border-color: var(--color-brand);
	font-weight: 600;
}
.mode-options button:hover {
	border-color: var(--color-brand);
}
.mode-dot {
	width: 15px;
	height: 15px;
	border: 1px solid #89919c;
	border-radius: 50%;
	background: #fff;
}
.mode-dot[data-theme='dark'] {
	background: #34363c;
}
.mode-dot[data-theme='oled'] {
	background: #000;
}
.mode-dot[data-theme='system'] {
	background: linear-gradient(90deg, #fff 50%, #34363c 50%);
}
.mode-dot[data-theme='retro'] {
	background: #1f604c;
}
.accent-controls,
.accent-swatches,
.custom-accent {
	display: flex;
	align-items: center;
	gap: 10px;
	flex-wrap: wrap;
}
.accent-controls {
	gap: 18px 24px;
}
.accent-swatch {
	display: grid;
	place-items: center;
	flex-shrink: 0;
	width: 32px;
	height: 32px;
	background: var(--swatch);
	color: #fff;
	border: 2px solid transparent;
	border-radius: 50%;
}
.accent-swatch[aria-pressed='true'] {
	outline: 2px solid var(--color-brand);
	outline-offset: 3px;
}
.accent-swatch svg {
	width: 19px;
	height: 19px;
}
.custom-accent {
	gap: 8px;
}
.custom-accent input[type='color'] {
	appearance: auto !important;
	width: 36px;
	min-height: 34px;
	height: 34px;
	padding: 2px;
	border: 1px solid var(--color-divider);
	border-radius: 6px;
	box-shadow: none;
	cursor: pointer;
}
.color-hex {
	width: 102px;
	min-height: 36px;
	padding: 6px 10px;
	border: 1px solid var(--color-divider);
	box-shadow: none;
	font-variant-numeric: tabular-nums;
}
.section-title-row {
	display: flex;
	align-items: center;
	justify-content: space-between;
	margin-bottom: 14px;
}
.section-title-row h2 {
	margin: 0;
}
.has-background {
	display: grid;
	grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr);
	column-gap: 24px;
}
.has-background .section-title-row {
	grid-column: 1 / -1;
}
.has-background .background-preview {
	grid-column: 1;
	grid-row: 2;
	height: 184px;
}
.has-background .background-controls {
	grid-column: 2;
	grid-row: 2;
	grid-template-columns: minmax(0, 1fr);
	gap: 6px;
	margin-top: 0;
}
.has-background .background-actions {
	grid-column: 1 / -1;
	grid-row: 3;
}
.has-background .preview-window {
	width: 86%;
}
.background-actions {
	display: flex;
	align-items: center;
	gap: 8px;
	flex-wrap: wrap;
	margin-top: 12px;
}
.background-status {
	margin-right: auto;
	color: var(--color-secondary);
	font-size: 14px;
}
.background-preview {
	isolation: isolate;
	position: relative;
	height: 168px;
	overflow: hidden;
	border-radius: 12px;
	background: var(--surface-2);
	display: grid;
	place-items: center;
}
.preview-photo {
	position: absolute;
	inset: -12px;
	background-size: cover;
	background-position: var(--gecl-background-position);
	filter: blur(var(--gecl-background-blur));
	z-index: -2;
}
.preview-overlay {
	position: absolute;
	inset: 0;
	background: var(--gecl-background-overlay);
	z-index: -1;
}
.background-disabled .preview-photo {
	visibility: hidden;
}
.preview-window {
	display: flex;
	width: 66%;
	max-width: 360px;
	border: 1px solid var(--color-divider);
	border-radius: 9px;
	overflow: hidden;
}
.preview-nav {
	display: flex;
	flex-direction: column;
	gap: 13px;
	align-items: center;
	padding: 16px 12px;
	background: var(--gecl-shell-surface);
}
.preview-nav span {
	width: 12px;
	height: 12px;
	background: var(--color-secondary);
	border-radius: 3px;
}
.preview-nav .preview-nav-active {
	background: var(--color-brand);
}
.preview-content {
	display: flex;
	flex-wrap: wrap;
	gap: 6px;
	padding: 16px;
	flex: 1;
	background: var(--gecl-content-surface);
}
.preview-content strong {
	color: var(--color-contrast);
	width: 100%;
	font-size: 15px;
}
.preview-detail {
	color: var(--color-secondary);
	font-size: 12px;
}
.preview-action {
	margin-left: auto;
	align-self: end;
	padding: 5px 9px;
	font-size: 12px;
	font-weight: 600;
	background: var(--color-brand);
	color: var(--color-accent-contrast);
	border-radius: 6px;
}
.background-controls {
	display: grid;
	grid-template-columns: 1fr 1fr;
	gap: 16px 32px;
	margin-top: 20px;
}
.range-control > span {
	display: flex;
	justify-content: space-between;
	gap: 12px;
	font-size: 14px;
}
output {
	color: var(--color-secondary);
	font-variant-numeric: tabular-nums;
}
input[type='range'] {
	width: 100%;
	min-height: 30px;
	margin: 4px 0 0;
	padding: 0;
	box-shadow: none;
	background: transparent;
	cursor: pointer;
}
input[type='range']::-webkit-slider-runnable-track {
	height: 4px;
	border-radius: 4px;
	background: var(--surface-5);
}
input[type='range']::-webkit-slider-thumb {
	appearance: none;
	width: 16px;
	height: 16px;
	margin-top: -6px;
	background: var(--color-brand);
	border-radius: 50%;
}
.position-control {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 12px;
	font-size: 14px;
}
select {
	padding: 7px 28px 7px 10px;
	min-height: 36px;
	color: var(--color-base);
	background: var(--surface-4);
	border: 1px solid var(--color-divider);
	border-radius: 8px;
}
.appearance-advanced {
	display: flex;
	flex-direction: column;
	gap: 18px;
}
.appearance-advanced > :last-child {
	align-self: flex-start;
}
.toggle-row {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 16px;
}
.appearance-error {
	display: flex;
	align-items: center;
	gap: 12px;
	margin-bottom: 16px;
	color: var(--color-red);
}
.retry-button {
	border: 0;
	background: transparent;
	color: var(--color-brand);
	text-decoration: underline;
}
@media (max-width: 1120px) {
	.background-controls {
		gap: 14px 22px;
	}
	.accent-controls {
		gap: 16px;
	}
}
</style>
