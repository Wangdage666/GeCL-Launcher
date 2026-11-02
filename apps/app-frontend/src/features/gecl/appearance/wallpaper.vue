<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch, watchEffect } from 'vue'

import { useTheme } from '@/composables/use-theme'

import { accentTokens, backdropTokens } from './palette'
import { visualAppearance } from './state'

const theme = useTheme()
const appearance = visualAppearance.active
const ready = ref(false)
const image = computed(() => {
	const { preferences, background } = appearance.value
	return preferences.backgroundEnabled && background?.id === preferences.backgroundId
		? background
		: null
})

watch(
	() => image.value?.path,
	() => {
		ready.value = false
	},
	{ flush: 'sync' },
)
watchEffect(() => {
	const root = document.documentElement
	const dark = theme.active !== 'light'
	for (const [key, value] of Object.entries({
		...accentTokens(appearance.value.preferences.accentColor, dark, !!image.value && ready.value),
		...backdropTokens(appearance.value.preferences, dark),
	}))
		root.style.setProperty(key, value)
	root.classList.toggle('gecl-has-background', !!image.value && ready.value)
})

onMounted(() => {
	void visualAppearance.load().catch(() => {})
})
</script>

<template>
	<div v-if="image" class="gecl-wallpaper" aria-hidden="true" :class="{ 'is-ready': ready }">
		<img
			:key="image.id"
			:src="convertFileSrc(image.path)"
			alt=""
			@load="ready = true"
			@error="ready = false"
		/>
	</div>
</template>

<style scoped>
.gecl-wallpaper {
	position: fixed;
	inset: 0;
	pointer-events: none;
	overflow: hidden;
	opacity: 0;
	transition: opacity 180ms ease-out;
}
.gecl-wallpaper.is-ready {
	opacity: 1;
}
.gecl-wallpaper img {
	width: 100%;
	height: 100%;
	object-fit: cover;
	object-position: var(--gecl-background-position);
	filter: blur(var(--gecl-background-blur));
	transform: scale(1.06);
}
.gecl-wallpaper::after {
	content: '';
	position: absolute;
	inset: 0;
	background: var(--gecl-background-overlay);
}
@media (prefers-reduced-motion: reduce) {
	.gecl-wallpaper {
		transition: none;
	}
}
</style>
