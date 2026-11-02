<script setup lang="ts">
import { useScrollIndicator } from '@modrinth/ui'
import { OverlayScrollbars } from 'overlayscrollbars'
import { computed, onMounted, onScopeDispose, ref } from 'vue'
import AccountsCard from '@/features/minecraft-accounts/accounts-card.vue'
import RecentAndDownloads from './recent-and-downloads.vue'
import { useTheme } from '@/composables/use-theme'
defineProps<{ visible: boolean }>()
const accounts = ref<InstanceType<typeof AccountsCard> | null>(null)
const minecraftAccounts = computed(() => accounts.value)
defineExpose({ minecraftAccounts })
const appTheme = useTheme()
const scrollContainer = ref<HTMLElement | null>(null)
const scrollViewport = ref<HTMLElement | null>(null)
const { showTopFade, showBottomFade, checkScrollState } = useScrollIndicator(scrollViewport)
let removeScrollbarListener: (() => void) | undefined
onMounted(() => {
	if (!scrollContainer.value) return
	const scrollbars = OverlayScrollbars(scrollContainer.value)
	if (!scrollbars) return
	scrollViewport.value = scrollbars.elements().viewport
	removeScrollbarListener = scrollbars.on('updated', checkScrollState)
})
onScopeDispose(() => removeScrollbarListener?.())
const scrollbarOptions = Object.freeze({ overflow: { x: 'hidden', y: 'scroll' } })
</script>

<template>
	<div
		v-show="visible"
		class="app-sidebar mt-px relative flex h-[calc(100vh-var(--top-bar-height))] w-[300px] shrink-0 flex-col overflow-visible border-0 border-l-[1px] border-solid border-[--brand-gradient-border] before:pointer-events-none before:absolute before:inset-y-0 before:-left-8 before:w-8 before:content-['']"
		:class="{
			'before:shadow-[-15px_0_15px_-15px_rgba(0,0,0,0.1)_inset]': appTheme.advancedRendering,
		}"
	>
		<div class="relative flex min-h-0 grow shrink flex-col">
			<div
				ref="scrollContainer"
				v-overlay-scrollbars="scrollbarOptions"
				class="app-sidebar-scrollable relative min-h-0 grow shrink"
				data-overlayscrollbars-initialize
			>
				<div id="sidebar-teleport-target" class="sidebar-teleport-content contents"></div>
                <div class="sidebar-default-content hidden" :class="{ 'sidebar-enabled': visible }">
                    <section class="p-4 border-0 border-b border-solid border-[--brand-gradient-border]">
                        <h2 class="m-0 text-base font-semibold text-contrast">游玩账户</h2>
                        <Suspense><AccountsCard ref="accounts" /></Suspense>
                    </section>
                    <RecentAndDownloads />
                </div>
			</div>
			<div
				class="sidebar-scroll-fade pointer-events-none absolute inset-x-0 top-0 z-10 h-8 rotate-180 transition-opacity duration-200 motion-reduce:transition-none"
				:class="showTopFade ? 'opacity-100' : 'opacity-0'"
				aria-hidden="true"
			/>
			<div
				class="sidebar-scroll-fade pointer-events-none absolute inset-x-0 bottom-0 z-10 h-8 transition-opacity duration-200 motion-reduce:transition-none"
				:class="showBottomFade ? 'opacity-100' : 'opacity-0'"
				aria-hidden="true"
			/>
		</div>
	</div>
</template>

<style scoped>
.app-sidebar {
	background: var(--brand-gradient-bg);
	--color-button-bg: var(--brand-gradient-button);
	--surface-4: var(--brand-gradient-button);
	--color-button-bg-hover: var(--brand-gradient-border);
	--surface-5: var(--brand-gradient-border);
	--color-divider: var(--brand-gradient-border);
	--color-divider-dark: var(--brand-gradient-border);
}

.sidebar-teleport-content:empty + .sidebar-default-content.sidebar-enabled {
	display: contents;
}

.sidebar-scroll-fade {
	background: var(--brand-gradient-fade-out-color);
}
</style>
