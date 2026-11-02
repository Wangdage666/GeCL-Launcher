<template>
	<NewModal ref="purchaseModal">
		<template #title>
			<span class="text-contrast text-xl font-extrabold">
				<template v-if="productType === 'midas'">订阅 Modrinth+</template>
				<template v-else-if="productType === 'pyro'">
					<template v-if="existingSubscription"> 升级服务器套餐 </template>
					<template v-else> 订阅 Modrinth 服务器托管 </template>
				</template>
				<template v-else>购买产品</template>
			</span>
		</template>
		<div class="flex items-center gap-1 pb-4">
			<template v-if="productType === 'pyro' && !projectId">
				<span
					:class="{
						'text-secondary': purchaseModalStep !== 0,
						'font-bold': purchaseModalStep === 0,
					}"
				>
					Configure
					<span class="hidden sm:inline">server</span>
				</span>
				<ChevronRightIcon class="h-5 w-5 text-secondary" />
			</template>
			<span
				:class="{
					'text-secondary': purchaseModalStep !== (productType === 'pyro' && !projectId ? 1 : 0),
					'font-bold': purchaseModalStep === (productType === 'pyro' && !projectId ? 1 : 0),
				}"
			>
				{{ productType === 'pyro' ? 'Billing' : 'Plan' }}
				<span class="hidden sm:inline">{{
					productType === 'pyro' ? 'interval' : 'selection'
				}}</span>
			</span>
			<ChevronRightIcon class="h-5 w-5 text-secondary" />
			<span
				:class="{
					'text-secondary': purchaseModalStep !== (productType === 'pyro' && !projectId ? 2 : 1),
					'font-bold': purchaseModalStep === (productType === 'pyro' && !projectId ? 2 : 1),
				}"
			>
				Payment
			</span>
			<ChevronRightIcon class="h-5 w-5 text-secondary" />
			<span
				:class="{
					'text-secondary': purchaseModalStep !== (productType === 'pyro' && !projectId ? 3 : 2),
					'font-bold': purchaseModalStep === (productType === 'pyro' && !projectId ? 3 : 2),
				}"
			> 核对订单 </span>
		</div>
		<div
			v-if="productType === 'pyro' && !projectId && purchaseModalStep === 0"
			class="md:w-[600px] flex flex-col gap-4"
		>
			<div v-if="!existingSubscription">
				<p class="my-2 text-lg font-bold">配置服务器</p>
				<div class="flex flex-col gap-4">
					<Input v-model="serverName" placeholder="服务器名称" :maxlength="48" />
					<!-- <DropdownSelect
            v-model="serverLoader"
            v-tooltip="'Select the mod loader for your server'"
            name="server-loader"
            :options="['Vanilla', 'Fabric', 'Forge']"
            placeholder="Select mod loader..."
          /> -->
					<div class="grid lg:grid-cols-5 grid-cols-3 gap-4">
						<button
							v-for="loader in [
								'Vanilla',
								'Fabric',
								'Forge',
								'Quilt',
								'NeoForge',
								'Paper',
								'Purpur',
							]"
							:key="loader"
							class="!h-24 btn flex !flex-col !items-center !justify-between !pt-4 !pb-3 !w-full"
							:style="{
								filter: serverLoader === loader ? 'brightness(1.5)' : '',
							}"
							@click="serverLoader = loader"
						>
							<TagIcon :tag="loader" enforce-type="loader" class="!h-12 !w-12" />
							<p class="text-lg font-bold m-0">{{ loader }}</p>
						</button>
					</div>
					<div class="flex items-center gap-2">
						<InfoIcon class="hidden sm:block" />
						<span class="text-sm text-secondary"> 之后可在服务器设置中修改。 </span>
					</div>
				</div>
			</div>
			<div v-if="customServer">
				<div class="flex gap-2 items-center">
					<p class="my-2 text-lg font-bold">
						<template v-if="existingSubscription">升级内存</template>
						<template v-else>配置内存</template>
					</p>
					<IssuesIcon
						v-if="customServerConfig.ramInGb < 4"
						v-tooltip="'This might not be powerful enough for your Minecraft server.'"
						class="h-6 w-6 text-orange"
					/>
				</div>
				<p v-if="existingPlan" class="mt-1 mb-2 text-secondary"> 当前套餐包含 <strong>{{ existingPlan.metadata.ram / 1024 }} GB 内存</strong> and
					<strong
						>{{ existingPlan.metadata.cpu / 2 }} 个共享 CPU（突发最高 {{ existingPlan.metadata.cpu }} 个 CPU）</strong
					>.
				</p>
				<div class="flex flex-col gap-4">
					<div class="flex w-full gap-2 items-center">
						<Slider
							v-model="customServerConfig.ramInGb"
							class="fix-slider"
							:min="customMinRam"
							:max="customMaxRam"
							:step="2"
							unit="GB"
						/>
						<div class="font-semibold text-nowrap"></div>
					</div>
					<div
						v-if="customMatchingProduct && !customOutOfStock"
						class="flex sm:flex-row flex-col gap-4 w-full"
					>
						<div class="flex flex-col w-full gap-2">
							<div class="font-semibold">共享 CPU</div>
							<Input :model-value="sharedCpus" disabled wrapper-class="w-full" />
						</div>
						<div class="flex flex-col w-full gap-2">
							<div class="font-semibold flex items-center gap-1"> 最大突发 CPU 数 <UnknownIcon
									v-tooltip="
										'CPU bursting allows your server to temporarily use additional threads to help mitigate TPS spikes. See Modrinth Hosting FAQ for more info.'
									"
									class="h-4 w-4text-secondary opacity-60"
								/>
							</div>
							<Input :model-value="mutatedProduct.metadata.cpu" disabled wrapper-class="w-full" />
						</div>
						<div class="flex flex-col w-full gap-2">
							<div class="font-semibold">Storage</div>
							<Input
								v-model="customServerConfig.storageGbFormatted"
								disabled
								wrapper-class="w-full"
							/>
						</div>
					</div>
					<Admonition
						v-else-if="customOutOfStock && customMatchingProduct"
						type="info"
						header="此套餐暂时售罄"
					> 当前 <a :href="outOfStockUrl" class="underline" target="_blank">资源不足</a> ，无法提供所选内存配置。请稍后重试，或选择其他配置。 </Admonition>
					<Admonition v-else type="info" header="未找到所选套餐"> 暂时没有符合所选内存配置的服务器。请稍后重试，或选择其他配置。 </Admonition>

					<div class="flex gap-2">
						<InfoIcon class="hidden sm:block shrink-0 mt-1" />
						<span class="text-sm text-secondary"> 存储空间和共享 CPU 数随内存配置分配，暂不支持单独调整。 </span>
					</div>
				</div>
			</div>
		</div>
		<div
			v-if="purchaseModalStep === (mutatedProduct.metadata.type === 'pyro' && !projectId ? 1 : 0)"
			class="md:w-[600px]"
		>
			<div>
				<p class="my-2 text-lg font-bold">选择计费周期</p>
				<div v-if="existingPlan" class="flex flex-col gap-3 mb-4 text-secondary">
					<p class="m-0"> 下方显示升级后的 <strong>续费价格</strong> 。 </p>
					<p class="m-0"> 今天将按当前计费周期剩余时长，收取相应比例的费用。 </p>
				</div>
				<div class="flex flex-col gap-4">
					<div
						v-for="([interval, rawPrice], index) in Object.entries(price.prices.intervals)"
						:key="index"
						class="flex cursor-pointer items-center gap-2"
						@click="selectedPlan = interval"
					>
						<RadioButtonCheckedIcon v-if="selectedPlan === interval" class="h-8 w-8 text-brand" />
						<RadioButtonIcon v-else class="h-8 w-8 text-secondary" />
						<span
							class="text-lg capitalize"
							:class="{ 'text-secondary': selectedPlan !== interval }"
						>
							{{ interval }}
						</span>
						<span
							v-if="interval === 'yearly' || interval === 'quarterly'"
							class="rounded-full bg-brand px-2 py-1 font-bold text-brand-inverted"
						>
							SAVE
							{{
								calculateSavings(
									price.prices.intervals.monthly,
									rawPrice,
									interval === 'quarterly' ? 3 : 12,
								)
							}}%
						</span>
						<span class="ml-auto text-lg" :class="{ 'text-secondary': selectedPlan !== interval }">
							{{ formatPrice(rawPrice, price.currency_code) }}
						</span>
					</div>
				</div>
				<div class="mt-4 flex justify-between border-0 border-t border-solid border-code-bg pt-4">
					<span class="text-xl text-secondary">Total</span>
					<div class="flex items-baseline gap-2">
						<span class="text-2xl font-extrabold text-primary">
							{{ formatPrice(price.prices.intervals[selectedPlan], price.currency_code) }}
						</span>
						<span class="text-lg text-secondary">/ {{ selectedPlan }}</span>
					</div>
				</div>

				<div class="flex items-center gap-2 pt-4">
					<InfoIcon class="hidden sm:block" />
					<span class="text-sm text-secondary"> 最终价格和币种取决于所选支付方式。 </span>
				</div>
			</div>
		</div>
		<template
			v-if="purchaseModalStep === (mutatedProduct.metadata.type === 'pyro' && !projectId ? 2 : 1)"
		>
			<div
				v-show="loadingPaymentMethodModal !== 2"
				class="flex min-h-[16rem] items-center justify-center md:w-[600px]"
			>
				<AnimatedLogo class="w-[80px]" />
			</div>
			<div v-show="loadingPaymentMethodModal === 2" class="min-h-[16rem] p-1 md:w-[600px]">
				<div id="address-element"></div>
				<div id="payment-element" class="mt-4"></div>
			</div>
		</template>
		<div
			v-if="purchaseModalStep === (mutatedProduct.metadata.type === 'pyro' && !projectId ? 3 : 2)"
			class="md:w-[650px]"
		>
			<div
				v-if="mutatedProduct.metadata.type === 'pyro' && !existingSubscription"
				class="r-4 rounded-xl bg-bg p-4 mb-4"
			>
				<p class="my-2 text-lg font-bold text-primary">服务器详情</p>
				<div class="flex items-center gap-4">
					<img
						v-if="projectImage"
						:src="projectImage"
						alt="项目图片"
						class="w-16 h-16 rounded"
					/>
					<div>
						<p v-if="projectName" class="font-bold">{{ projectName }}</p>
						<p>服务器名称： {{ serverName }}</p>
						<p v-if="!projectId">加载器： {{ serverLoader }}</p>
					</div>
				</div>
			</div>
			<div>
				<div class="r-4 rounded-xl bg-bg p-4">
					<p class="my-2 text-lg font-bold text-primary">订单详情</p>
					<div class="mb-2 flex justify-between">
						<span class="text-secondary">
							{{ mutatedProduct.metadata.type === 'midas' ? 'Modrinth+' : 'Modrinth Hosting' }}
							{{
								existingPlan
									? `(${dayjs(renewalDate).diff(dayjs(), 'days')} days prorated)`
									: selectedPlan
							}}
						</span>
						<span v-if="existingPlan" class="text-secondary text-end">
							{{ formatPrice(total - tax, price.currency_code) }}
						</span>
						<span v-else class="text-secondary text-end">
							{{ formatPrice(total - tax, price.currency_code) }} /
							{{ selectedPlan }}
						</span>
					</div>
					<div class="flex justify-between">
						<span class="text-secondary">Tax</span>
						<span class="text-secondary text-end">{{ formatPrice(tax, price.currency_code) }}</span>
					</div>
					<div class="mt-4 flex justify-between border-0 border-t border-solid border-code-bg pt-4">
						<span class="text-lg font-bold">今日应付</span>
						<span class="text-lg font-extrabold text-primary text-end">
							{{ formatPrice(total, price.currency_code) }}
						</span>
					</div>
				</div>
				<p class="my-2 text-lg font-bold">支付方式</p>
				<Combobox
					v-model="selectedPaymentMethodId"
					placeholder="支付方式"
					:options="selectablePaymentMethodOptions"
					:searchable="false"
					:show-icon-in-selected="true"
					force-direction="up"
					class="max-w-[20rem]"
				/>
			</div>
			<p class="m-0 mt-9 text-sm text-secondary">
				<strong>点击“订阅”即表示购买自动续费订阅。</strong>
				<br /> 将收取 {{ formatPrice(price.prices.intervals[selectedPlan], price.currency_code) }}
				/ {{ selectedPlan }} 及适用税费，开始时间： {{ existingPlan ? formatDate(renewalDate) : 'today' }}，直至取消订阅。 <br /> 可随时在设置页取消订阅。 </p>
			<p v-if="mutatedProduct.metadata.type === 'pyro'" class="mb-2 mt-4 text-secondary">
				<Checkbox v-model="eulaAccepted" :disabled="paymentLoading">
					<label> 我已阅读并同意 <a class="underline" target="_blank" href="https://aka.ms/MinecraftEULA"> Minecraft 最终用户许可协议 </a>
					</label>
				</Checkbox>
			</p>
		</div>
		<div class="input-group push-right pt-4">
			<template v-if="purchaseModalStep === 0">
				<button class="btn" @click="$refs.purchaseModal.hide()">
					<XIcon /> 取消 </button>
				<button
					class="btn btn-primary"
					:disabled="
						paymentLoading ||
						(mutatedProduct.metadata.type === 'pyro' && !projectId && !serverName) ||
						customNotAllowedToContinue ||
						upgradeNotAllowedToContinue
					"
					@click="nextStep"
				>
					<template v-if="customServer && customLoading">
						<SpinnerIcon class="animate-spin" /> 正在检查可用资源… </template>
					<template v-else>
						<RightArrowIcon />
						{{ mutatedProduct.metadata.type === 'pyro' && !projectId ? 'Next' : 'Select' }}
					</template>
				</button>
			</template>
			<template
				v-else-if="
					purchaseModalStep === (mutatedProduct.metadata.type === 'pyro' && !projectId ? 1 : 0)
				"
			>
				<button
					class="btn"
					@click="
						purchaseModalStep =
							mutatedProduct.metadata.type === 'pyro' && !projectId ? 0 : purchaseModalStep
					"
				>
					Back
				</button>
				<button class="btn btn-primary" :disabled="paymentLoading" @click="beginPurchaseFlow(true)">
					<RightArrowIcon />
					Select
				</button>
			</template>
			<template
				v-else-if="
					purchaseModalStep === (mutatedProduct.metadata.type === 'pyro' && !projectId ? 2 : 1)
				"
			>
				<button
					class="btn"
					@click="
						() => {
							purchaseModalStep = mutatedProduct.metadata.type === 'pyro' && !projectId ? 1 : 0
							loadingPaymentMethodModal = 0
							paymentLoading = false
						}
					"
				>
					Back
				</button>
				<button class="btn btn-primary" :disabled="paymentLoading" @click="validatePayment">
					<RightArrowIcon />
					Continue
				</button>
			</template>
			<template
				v-else-if="
					purchaseModalStep === (mutatedProduct.metadata.type === 'pyro' && !projectId ? 3 : 2)
				"
			>
				<button class="btn" @click="$refs.purchaseModal.hide()">
					<XIcon /> 取消 </button>
				<button
					v-if="mutatedProduct.metadata.type === 'pyro'"
					class="btn btn-primary"
					:disabled="paymentLoading || !eulaAccepted"
					@click="submitPayment"
				>
					<CheckCircleIcon />
					Subscribe
				</button>
				<!-- Default Subscribe Button, so M+ still works -->
				<button v-else class="btn btn-primary" :disabled="paymentLoading" @click="submitPayment">
					<CheckCircleIcon />
					Subscribe
				</button>
			</template>
		</div>
	</NewModal>
</template>

<script setup>
import {
	CardIcon,
	CheckCircleIcon,
	ChevronRightIcon,
	CurrencyIcon,
	InfoIcon,
	IssuesIcon,
	PayPalIcon,
	PlusIcon,
	RadioButtonCheckedIcon,
	RadioButtonIcon,
	RightArrowIcon,
	SpinnerIcon,
	UnknownIcon,
	XIcon,
} from '@modrinth/assets'
import { calculateSavings, createStripeElements, getCurrency } from '@modrinth/utils'
import dayjs from 'dayjs'
import { computed, nextTick, reactive, ref, watch } from 'vue'

import { useDebugLogger } from '../../composables/debug-logger'
import { useVIntl } from '../../composables/i18n'
import { useFormatDateTime, useFormatPrice } from '../../composables/index.ts'
import { paymentMethodMessages } from '../../utils/common-messages'
import Admonition from '../base/Admonition.vue'
import Checkbox from '../base/Checkbox.vue'
import Combobox from '../base/Combobox.vue'
import Input from '../base/inputs/Input.vue'
import Slider from '../base/Slider.vue'
import TagIcon from '../base/TagIcon.vue'
import AnimatedLogo from '../brand/AnimatedLogo.vue'
import NewModal from '../modal/NewModal.vue'

const { formatMessage } = useVIntl()
const formatPrice = useFormatPrice()
const formatDate = useFormatDateTime({ dateStyle: 'long' })
const debug = useDebugLogger('PurchaseModal')

const props = defineProps({
	product: {
		type: Object,
		required: true,
	},
	customer: {
		type: Object,
		required: true,
	},
	paymentMethods: {
		type: Array,
		required: true,
	},
	country: {
		type: String,
		required: true,
	},
	returnUrl: {
		type: String,
		required: true,
	},
	publishableKey: {
		type: String,
		required: true,
	},
	fetchPaymentData: {
		type: Function,
		default: async () => {},
	},
	sendBillingRequest: {
		type: Function,
		required: true,
	},
	onError: {
		type: Function,
		required: true,
	},
	projectName: {
		type: String,
		default: null,
	},
	projectImage: {
		type: String,
		default: null,
	},
	projectId: {
		type: String,
		default: null,
	},
	versionId: {
		type: String,
		default: null,
	},
	serverName: {
		type: String,
		default: null,
	},
	customServer: {
		type: Boolean,
		required: false,
	},
	fetchCapacityStatuses: {
		type: Function,
		required: false,
		default: null,
	},
	outOfStockUrl: {
		type: String,
		required: false,
		default: '',
	},
	existingSubscription: {
		type: Object,
		required: false,
		default: null,
	},
	existingPlan: {
		type: Object,
		required: false,
		default: null,
	},
	renewalDate: {
		type: String,
		required: false,
		default: null,
	},
})

const productType = computed(() => (props.customServer ? 'pyro' : props.product.metadata.type))

let stripe = null
let elements = null

const purchaseModal = ref()
const purchaseModalStep = ref(0)
const loadingPaymentMethodModal = ref(0)
const paymentLoading = ref(false)

const selectedPlan = ref('yearly')
const currency = computed(() => getCurrency(props.country))
const price = computed(() => {
	return (
		mutatedProduct.value?.prices?.find((x) => x.currency_code === currency.value) ??
		mutatedProduct.value?.prices?.find((x) => x.currency_code === 'USD') ??
		null
	)
})

const clientSecret = ref()
const paymentIntentId = ref()
const confirmationToken = ref()
const tax = ref()
const total = ref()

const serverName = ref(props.serverName || '')
const serverLoader = ref('Vanilla')
const eulaAccepted = ref(!!props.existingSubscription)

const mutatedProduct = ref({ ...props.product })
const customMinRam = ref(0)
const customMaxRam = ref(0)
const customMatchingProduct = ref()
const customOutOfStock = ref(false)
const customLoading = ref(true)
const customNotAllowedToContinue = computed(
	() =>
		props.customServer &&
		!props.existingSubscription &&
		(!customMatchingProduct.value || customLoading.value || customOutOfStock.value),
)
const upgradeNotAllowedToContinue = computed(
	() => props.existingSubscription && (customOutOfStock.value || customLoading.value),
)

const customServerConfig = reactive({
	ramInGb: 4,
	storageGbFormatted: computed(() => `${mutatedProduct.value.metadata.storage / 1024} GB`),
	ram: computed(() => customServerConfig.ramInGb * 1024),
})

const updateCustomServerProduct = () => {
	customMatchingProduct.value = props.product.find(
		(product) => product.metadata.ram === customServerConfig.ram,
	)

	if (customMatchingProduct.value) {
		mutatedProduct.value = { ...customMatchingProduct.value }
	}
}

let updateCustomServerStockTimeout = null
const updateCustomServerStock = async () => {
	if (updateCustomServerStockTimeout) {
		clearTimeout(updateCustomServerStockTimeout)
		customLoading.value = true
	}

	updateCustomServerStockTimeout = setTimeout(async () => {
		if (props.fetchCapacityStatuses) {
			if (props.existingSubscription) {
				if (mutatedProduct.value) {
					const capacityStatus = await props.fetchCapacityStatuses(
						props.existingSubscription.metadata.id,
						mutatedProduct.value,
					)
					customOutOfStock.value = capacityStatus.custom?.available === 0
					debug(capacityStatus)
				}
			} else {
				const capacityStatus = await props.fetchCapacityStatuses(mutatedProduct.value)
				customOutOfStock.value = capacityStatus.custom?.available === 0
			}
		} else {
			console.error('No fetchCapacityStatuses function provided.')
			customOutOfStock.value = true
		}
		customLoading.value = false
	}, 300)
}

function updateRamValues() {
	const ramValues = props.product.map((product) => product.metadata.ram / 1024)
	customMinRam.value = Math.min(...ramValues)
	customMaxRam.value = Math.max(...ramValues)

	if (props.product.some((product) => product.metadata.ram / 1024 === 4)) {
		customServerConfig.ramInGb = 4
	} else {
		customServerConfig.ramInGb = customMinRam.value
	}
}

if (props.customServer) {
	updateRamValues()

	const updateProductAndStock = () => {
		updateCustomServerProduct()
		updateCustomServerStock()
	}

	updateProductAndStock()
	watch(() => customServerConfig.ram, updateProductAndStock)
}

const selectedPaymentMethod = ref()
const inputtedPaymentMethod = ref()
const selectablePaymentMethods = computed(() => {
	const values = [
		...(props.paymentMethods ?? []),
		{
			id: 'new',
		},
	]

	if (inputtedPaymentMethod.value) {
		values.unshift(inputtedPaymentMethod.value)
	}

	return values
})

function formatPaymentMethodLabel(paymentMethod) {
	if (!paymentMethod) {
		return formatMessage(paymentMethodMessages.unknown)
	}

	if (paymentMethod.id === 'new') {
		return "添加支付方式"
	}

	if (paymentMethod.type === 'card') {
		return formatMessage(paymentMethodMessages.paymentMethodCardDisplay, {
			card_brand:
				formatMessage(paymentMethodMessages[paymentMethod.card?.brand]) ??
				formatMessage(paymentMethodMessages.unknown),
			last_four: paymentMethod.card?.last4 ?? '****',
		})
	}

	const typeLabel =
		formatMessage(paymentMethodMessages[paymentMethod.type]) ??
		formatMessage(paymentMethodMessages.unknown)
	let suffix = ''

	if (paymentMethod.type === 'cashapp' && paymentMethod.cashapp?.cashtag) {
		suffix = ` (${paymentMethod.cashapp.cashtag})`
	} else if (paymentMethod.type === 'paypal' && paymentMethod.paypal?.payer_email) {
		suffix = ` (${paymentMethod.paypal.payer_email})`
	}

	return `${typeLabel}${suffix}`
}

function getPaymentMethodIcon(paymentMethod) {
	if (paymentMethod.id === 'new') return PlusIcon
	if (paymentMethod.type === 'card') return CardIcon
	if (paymentMethod.type === 'cashapp') return CurrencyIcon
	if (paymentMethod.type === 'paypal') return PayPalIcon
	return undefined
}

const selectablePaymentMethodOptions = computed(() =>
	selectablePaymentMethods.value.map((paymentMethod) => ({
		value: paymentMethod.id,
		label: formatPaymentMethodLabel(paymentMethod),
		icon: getPaymentMethodIcon(paymentMethod),
	})),
)

const selectedPaymentMethodId = computed({
	get: () => selectedPaymentMethod.value?.id ?? null,
	set: (value) => {
		if (!value) return

		const paymentMethod = selectablePaymentMethods.value.find((method) => method.id === value)
		if (paymentMethod) {
			selectedPaymentMethod.value = paymentMethod
			void selectPaymentMethod(paymentMethod)
		}
	},
})

const primaryPaymentMethodId = computed(() => {
	if (
		props.customer &&
		props.customer.invoice_settings &&
		props.customer.invoice_settings.default_payment_method
	) {
		return props.customer.invoice_settings.default_payment_method
	} else if (props.paymentMethods?.length === 1) {
		return props.paymentMethods[0].id
	} else {
		return null
	}
})

const metadata = computed(() => {
	if (mutatedProduct.value.metadata.type === 'pyro') {
		return {
			type: 'pyro',
			server_name: serverName.value,
			source:
				props.projectId && props.versionId
					? {
							project_id: props.projectId,
							version_id: props.versionId,
						}
					: {
							loader: serverLoader.value,
							loader_version: '',
							game_version: 'latest',
						},
		}
	}
	return null
})

const sharedCpus = computed(() => {
	return (mutatedProduct.value?.metadata?.cpu ?? 0) / 2
})

function nextStep() {
	if (
		mutatedProduct.value.metadata.type === 'pyro' &&
		!props.projectId &&
		purchaseModalStep.value === 0
	) {
		purchaseModalStep.value = 1
	} else {
		beginPurchaseFlow(true)
	}
}

async function beginPurchaseFlow(skip = false) {
	if (!props.customer) {
		paymentLoading.value = true
		await props.fetchPaymentData()
		paymentLoading.value = false
	}

	if (primaryPaymentMethodId.value && skip) {
		paymentLoading.value = true
		await refreshPayment(null, primaryPaymentMethodId.value)
		paymentLoading.value = false
		purchaseModalStep.value =
			mutatedProduct.value.metadata.type === 'pyro' && !props.projectId ? 3 : 2
	} else {
		try {
			loadingPaymentMethodModal.value = 0
			purchaseModalStep.value =
				mutatedProduct.value.metadata.type === 'pyro' && !props.projectId ? 2 : 1

			await nextTick()

			const {
				elements: elementsVal,
				addressElement,
				paymentElement,
			} = createStripeElements(stripe, props.paymentMethods, {
				mode: 'payment',
				amount: price.value.prices.intervals[selectedPlan.value],
				currency: price.value.currency_code.toLowerCase(),
				paymentMethodCreation: 'manual',
				setupFutureUsage: 'off_session',
			})

			elements = elementsVal
			paymentElement.on('ready', () => {
				loadingPaymentMethodModal.value += 1
			})
			addressElement.on('ready', () => {
				loadingPaymentMethodModal.value += 1
			})
		} catch (err) {
			props.onError(err)
		}
	}
}

async function createConfirmationToken() {
	const { error, confirmationToken: confirmation } = await stripe.createConfirmationToken({
		elements,
	})

	if (error) {
		props.onError(error)
		return
	}

	return confirmation.id
}

async function validatePayment() {
	paymentLoading.value = true
	const { error: submitError } = await elements.submit()

	if (submitError) {
		paymentLoading.value = false
		props.onError(submitError)
		return
	}

	await refreshPayment(await createConfirmationToken())

	elements.update({ currency: price.value.currency_code.toLowerCase(), amount: total.value })

	loadingPaymentMethodModal.value = 0
	confirmationToken.value = await createConfirmationToken()
	purchaseModalStep.value =
		mutatedProduct.value.metadata.type === 'pyro' && !props.projectId ? 3 : 2
	paymentLoading.value = false
}

async function selectPaymentMethod(paymentMethod) {
	if (paymentMethod.id === 'new') {
		await beginPurchaseFlow(false)
	} else if (inputtedPaymentMethod.value && inputtedPaymentMethod.value.id === paymentMethod.id) {
		paymentLoading.value = true
		await refreshPayment(confirmationToken.value)
		paymentLoading.value = false
	} else {
		paymentLoading.value = true
		await refreshPayment(null, paymentMethod.id)
		paymentLoading.value = false
	}
}

async function refreshPayment(confirmationId, paymentMethodId) {
	try {
		const base = confirmationId
			? {
					type: 'confirmation_token',
					token: confirmationId,
				}
			: {
					type: 'payment_method',
					id: paymentMethodId,
				}

		const result = await props.sendBillingRequest(
			props.existingSubscription
				? {
						interval: selectedPlan.value,
						cancelled: false,
						product: mutatedProduct.value.id,
						payment_method: paymentMethodId,
					}
				: {
						charge: {
							type: 'new',
							product_id: mutatedProduct.value.id,
							interval: selectedPlan.value,
						},
						existing_payment_intent: paymentIntentId.value,
						metadata: metadata.value,
						...base,
					},
		)

		if (!paymentIntentId.value) {
			paymentIntentId.value = result.payment_intent_id
			clientSecret.value = result.client_secret
		}

		price.value = mutatedProduct.value.prices.find((x) => x.id === result.price_id)
		currency.value = price.value.currency_code
		tax.value = result.tax
		total.value = result.total

		if (confirmationId) {
			confirmationToken.value = confirmationId
			if (result.payment_method) {
				inputtedPaymentMethod.value = result.payment_method
			}
		}

		if (result.payment_method) {
			selectedPaymentMethod.value = result.payment_method
		}
	} catch (err) {
		props.onError(err)
	}
}

async function submitPayment() {
	paymentLoading.value = true
	const { error } = await stripe.confirmPayment({
		clientSecret: clientSecret.value,
		confirmParams: {
			confirmation_token: confirmationToken.value,
			return_url: `${props.returnUrl}?priceId=${price.value.id}&plan=${selectedPlan.value}`,
		},
	})

	if (error) {
		props.onError(error)
	}
	paymentLoading.value = false
}

defineExpose({
	show: () => {
		if (props.customServer) {
			updateRamValues()
		}

		stripe = Stripe(props.publishableKey)

		selectedPlan.value = props.existingSubscription ? props.existingSubscription.interval : 'yearly'
		serverName.value = props.serverName || ''
		serverLoader.value = 'Vanilla'

		purchaseModalStep.value = 0
		loadingPaymentMethodModal.value = 0
		paymentLoading.value = false

		purchaseModal.value.show()
	},
})
</script>
<style scoped lang="scss"></style>
