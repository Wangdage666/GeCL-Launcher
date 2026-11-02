<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import { ChevronRightIcon, ExternalIcon, XIcon } from '@modrinth/assets'
import { computed, ref, useTemplateRef } from 'vue'

import { Button, IconButton } from '#ui/components/base/buttons'

import NewModal from '../modal/NewModal.vue'
import type { ServerBillingInterval } from './ModrinthServersPurchaseModal.vue'
import PlanSelector from './ServersPurchase0Plan.vue'

const props = withDefaults(
	defineProps<{
		availableProducts: Labrinth.Billing.Internal.Product[]
		currency: string
		loggedIn?: boolean
	}>(),
	{
		loggedIn: false,
	},
)

const emit = defineEmits<{
	(e: 'continue', payload: { interval: ServerBillingInterval; planId: string | null }): void
}>()

const modal = useTemplateRef<InstanceType<typeof NewModal>>('modal')

const selectedPlan = ref<Labrinth.Billing.Internal.Product>()
const selectedInterval = ref<ServerBillingInterval>('quarterly')
const showSignInPrompt = ref(false)

const defaultPlan = computed<Labrinth.Billing.Internal.Product | undefined>(() => {
	return (
		props.availableProducts.find((p) => p?.metadata?.type === 'pyro' && p.metadata.ram === 6144) ??
		props.availableProducts.find((p) => p?.metadata?.type === 'pyro') ??
		props.availableProducts[0]
	)
})

function emitSelection() {
	emit('continue', {
		interval: selectedInterval.value,
		planId: selectedPlan.value?.id ?? null,
	})
}

function continueWithSelection() {
	if (!props.loggedIn) {
		showSignInPrompt.value = true
		return
	}

	emitSelection()
	modal.value?.hide()
}

function chooseCustom() {
	selectedPlan.value = undefined
}

function closeSignInPrompt() {
	showSignInPrompt.value = false
}

function continueToAuth() {
	emitSelection()
	closeSignInPrompt()
	modal.value?.hide()
}

function handleModalHide() {
	closeSignInPrompt()
}

function show(initialInterval?: ServerBillingInterval, initialPlanId?: string | null) {
	closeSignInPrompt()
	selectedInterval.value = initialInterval ?? 'quarterly'
	if (initialPlanId === null) {
		selectedPlan.value = undefined
	} else if (initialPlanId) {
		selectedPlan.value = props.availableProducts.find((product) => product.id === initialPlanId)
	} else {
		selectedPlan.value = defaultPlan.value
	}
	modal.value?.show()
}

defineExpose({
	show,
})
</script>

<template>
	<NewModal ref="modal" :on-hide="handleModalHide" no-padding>
		<template #title>
			<div class="flex items-center gap-1 font-bold text-secondary">
				<span class="text-contrast">Plan</span>
				<ChevronRightIcon class="h-5 w-5 text-secondary" stroke-width="3" />
				<span class=""> 地区 </span>
				<ChevronRightIcon class="h-5 w-5 text-secondary" stroke-width="3" />
				<span class=""> 支付方式 </span>
				<ChevronRightIcon class="h-5 w-5 text-secondary" stroke-width="3" />
				<span class=""> 核对订单 </span>
			</div>
		</template>
		<div class="relative w-[56rem] max-w-full">
			<div class="w-full h-full p-6">
				<PlanSelector
					v-model:plan="selectedPlan"
					v-model:interval="selectedInterval"
					:available-products="availableProducts"
					:currency="currency"
					@choose-custom="chooseCustom"
					@proceed="continueWithSelection"
				/>
			</div>

			<Transition
				enter-active-class="transition-opacity duration-150 ease-out"
				enter-from-class="opacity-0"
				enter-to-class="opacity-100"
				leave-active-class="transition-opacity duration-150 ease-in"
				leave-from-class="opacity-100"
				leave-to-class="opacity-0"
			>
				<div
					v-if="showSignInPrompt"
					class="pointer-events-auto absolute inset-0 z-20 bg-black/60"
					@click="closeSignInPrompt"
				/>
			</Transition>

			<Transition
				enter-active-class="transition-all duration-200 ease-out"
				enter-from-class="translate-y-4 opacity-0"
				enter-to-class="translate-y-0 opacity-100"
				leave-active-class="transition-all duration-150 ease-in"
				leave-from-class="translate-y-0 opacity-100"
				leave-to-class="translate-y-4 opacity-0"
			>
				<div
					v-if="showSignInPrompt"
					class="absolute inset-x-0 bottom-0 -m-px z-30 rounded-2xl border border-solid border-surface-5 bg-bg-raised p-6 shadow-2xl"
				>
					<div class="absolute right-4 top-4">
						<IconButton type="quiet" label="关闭登录提示" @click="closeSignInPrompt">
							<XIcon />
						</IconButton>
					</div>

					<div class="mx-auto flex max-w-xl flex-col items-center gap-4 text-center">
						<div class="flex flex-col gap-2">
							<div class="font-semibold text-contrast">登录后继续购买</div>
							<div class="">添加账单信息需要登录 Modrinth 账户。</div>
						</div>
						<Button type="colored" color="brand" class="mt-2" @click="continueToAuth"> 登录或创建账户 <ExternalIcon class="size-4" />
						</Button>
					</div>
				</div>
			</Transition>
		</div>
	</NewModal>
</template>
