<script setup>
import {
	CheckIcon,
	CopyIcon,
	DropdownIcon,
	FolderOpenIcon,
	HammerIcon,
	LogInIcon,
	UpdatedIcon,
	WrenchIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Button,
	ButtonLink,
	Collapsible,
	defineMessages,
	IconButton,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { ChatIcon } from '@/assets/icons'
import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import { handleSevereError } from '@/composables/use-error.js'
import { trackEvent } from '@/helpers/analytics'
import { injectMinecraftAccounts } from '@/features/minecraft-accounts/context'
import { install_existing_instance } from '@/helpers/install'
import { cancel_directory_change } from '@/helpers/settings.ts'
import { showAppDbBackupsFolder } from '@/helpers/utils.js'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	openBackupsFolder: {
		id: 'app.error.state-init.open-backups-folder',
		defaultMessage: 'Open backups folder',
	},
})

const errorModal = ref()
const error = ref()
const closable = ref(true)
const errorCollapsed = ref(false)

const title = ref("发生错误")
const errorType = ref('unknown')
const supportLink = ref('https://support.modrinth.com')
const metadata = ref({})

defineExpose({
	async show(errorVal, context, canClose = true, source = null) {
		closable.value = canClose

		if (errorVal.code?.startsWith('minecraft_auth_')) {
			title.value = "无法登录 Minecraft"
			errorType.value = 'minecraft_auth'
			supportLink.value =
				'https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues'

			if (errorVal.code === 'minecraft_auth_network') {
				metadata.value.network = true
			}
			if (errorVal.code === 'minecraft_auth_connection_refused') {
				metadata.value.hostsFile = true
			}
		} else if (errorVal.code?.startsWith('directory_move_')) {
			title.value = "无法更改应用目录"
			errorType.value = 'directory_move'
			supportLink.value = 'https://support.modrinth.com'

			if (errorVal.code === 'directory_move_permission_denied') {
				metadata.value.readOnly = true
			}

			if (errorVal.code === 'directory_move_no_space') {
				metadata.value.notEnoughSpace = true
			}
		} else if (errorVal.code === 'no_loader_version') {
			title.value = "未选择加载器"
			errorType.value = 'no_loader_version'
			supportLink.value = 'https://support.modrinth.com'
			metadata.value.instanceId = context.instanceId
		} else if (source === 'state_init') {
			title.value = "gecl 初始化失败"
			errorType.value = 'state_init'
			supportLink.value = 'https://support.modrinth.com'
		} else {
			title.value = "发生错误"
			errorType.value = 'unknown'
			supportLink.value = 'https://support.modrinth.com'
			metadata.value = {}
		}

		error.value = errorVal
		errorModal.value.show()
	},
})

const minecraftAccounts = injectMinecraftAccounts()
const loadingMinecraft = ref(false)
async function loginMinecraft() {
	try {
		loadingMinecraft.value = true
		errorModal.value.hide()
		await minecraftAccounts.value?.login()

		await trackEvent('AccountLogIn', { source: 'ErrorModal' })
		loadingMinecraft.value = false
		errorModal.value.hide()
	} catch (err) {
		loadingMinecraft.value = false
		handleSevereError(err)
	}
}

async function cancelDirectoryChange() {
	try {
		await cancel_directory_change()
		window.location.reload()
	} catch (err) {
		handleError(err)
	}
}

function retryDirectoryChange() {
	window.location.reload()
}

async function openDbBackupsFolder() {
	await showAppDbBackupsFolder().catch(handleError)
}

const loadingRepair = ref(false)
async function repairInstance() {
	loadingRepair.value = true
	try {
		await install_existing_instance(metadata.value.instanceId, false)
		errorModal.value.hide()
	} catch (err) {
		handleSevereError(err)
	}
	loadingRepair.value = false
}

const hasDebugInfo = computed(
	() =>
		errorType.value === 'directory_move' ||
		errorType.value === 'minecraft_auth' ||
		errorType.value === 'state_init' ||
		errorType.value === 'no_loader_version',
)

const debugInfo = computed(() => error.value.message ?? error.value ?? "未提供错误信息。")

const copied = ref(false)

async function copyToClipboard(text) {
	await navigator.clipboard.writeText(text)
	copied.value = true
	setTimeout(() => {
		copied.value = false
	}, 3000)
}
</script>

<template>
	<ModalWrapper ref="errorModal" :header="title" :closable="closable">
		<div class="modal-body max-w-[550px]">
			<div class="markdown-body">
				<template v-if="errorType === 'minecraft_auth'">
					<template v-if="metadata.network">
						<h3>网络连接异常</h3>
						<p> gecl 无法连接微软服务器。请检查网络后重试；若仍然失败，可参阅 <a
								href="https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues#h_e71a5f805f"
							> 上游故障排查文档 </a> 进行排查。 </p>
					</template>
					<template v-else-if="metadata.hostsFile">
						<h3>网络连接异常</h3>
						<p> 连接 Microsoft、Xbox 或 Minecraft 服务时，请求被拒绝。请检查防火墙、代理和 hosts 文件，也可参阅 <a
								href="https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues#h_d694a29256"
							> 上游故障排查文档 </a> 了解处理步骤。 </p>
					</template>
					<template v-else>
						<h3>尝试其他微软账户</h3>
						<p> 请确认当前账户是否拥有 Minecraft；你可能使用了另一个微软账户购买游戏。 </p>
						<div class="cta-button">
							<button class="btn btn-primary" :disabled="loadingMinecraft" @click="loginMinecraft">
								<LogInIcon /> 更换账户 </button>
						</div>
						<h3>使用 PC Game Pass、从基岩版转来，或刚购买游戏？</h3>
						<p> 请先登录 <a href="https://www.minecraft.net/en-us/download">Minecraft 官方启动器</a> ，完成账户设置后再回到 gecl 登录。 </p>
					</template>
					<div class="cta-button">
						<button class="btn btn-primary" :disabled="loadingMinecraft" @click="loginMinecraft">
							<LogInIcon /> 重新登录 </button>
					</div>
				</template>
				<template v-if="errorType === 'directory_move'">
					<template v-if="metadata.readOnly">
						<h3>检查目录权限</h3>
						<p> gecl 无法写入所选目录。请调整目录权限后重试，或取消更改目录。 </p>
					</template>
					<template v-else-if="metadata.notEnoughSpace">
						<h3>磁盘空间不足</h3>
						<p> 所选目录所在磁盘的空间不足。请释放空间后重试，或取消更改目录。 </p>
					</template>
					<template v-else>
						<p> 无法将数据迁移到所选目录。请保留诊断信息以便排查，或取消更改目录。 </p>
					</template>

					<div class="cta-button">
						<button class="btn" @click="retryDirectoryChange">
							<UpdatedIcon /> 重试迁移 </button>
						<button class="btn btn-danger" @click="cancelDirectoryChange">
							<XIcon /> 取消迁移 </button>
					</div>
				</template>
				<template v-else-if="errorType === 'state_init'">
					<p> gecl 加载失败，可能有文件损坏或关键文件缺失。 </p>
					<p>可尝试以下方法：</p>
					<ul>
						<li>检查网络连接，然后重启应用。</li>
						<li>重新下载应用。</li>
					</ul>
				</template>
				<template v-else-if="errorType === 'no_loader_version'">
					<p>未找到此游戏版本的加载器版本。</p>
					<p>请点击下方按钮修复游戏版本。</p>
					<div class="cta-button">
						<button class="btn btn-primary" :disabled="loadingRepair" @click="repairInstance">
							<HammerIcon /> 修复游戏版本 </button>
					</div>
				</template>
				<template v-else>
					{{ debugInfo }}
				</template>
				<template v-if="hasDebugInfo">
					<div class="w-full h-[1px] bg-surface-5 mb-3"></div>
					<p> 若问题仍未解决，可参考 <a :href="supportLink">上游支持页面</a> 。gecl 是独立衍生版；反馈问题时请说明版本，并附上下方诊断信息。 </p>
				</template>
			</div>
			<div class="flex items-center gap-2">
				<ButtonLink :href="supportLink" @click="errorModal.hide()"
					><ChatIcon /> 查看上游帮助</ButtonLink
				>
				<Button v-if="closable" @click="errorModal.hide()"><XIcon /> 关闭</Button>
			</div>
			<template v-if="hasDebugInfo">
				<div class="flex flex-col gap-2">
					<div class="w-full h-[1px] bg-surface-5"></div>

					<div class="overflow-clip">
						<button
							class="flex items-center justify-between w-full bg-transparent border-0 py-4 cursor-pointer"
							@click="errorCollapsed = !errorCollapsed"
						>
							<span class="flex items-center gap-2 text-contrast font-extrabold m-0">
								<WrenchIcon class="h-4 w-4" /> 诊断信息 </span>
							<DropdownIcon
								class="h-5 w-5 text-secondary transition-transform"
								:class="{ 'rotate-180': !errorCollapsed }"
							/>
						</button>
						<Collapsible :collapsed="errorCollapsed">
							<div
								class="p-3 bg-surface-2 rounded-2xl text-xs grid grid-cols-[1fr_auto] max-w-full items-start"
							>
								<div
									class="m-0 p-0 rounded-none bg-transparent text-sm font-mono break-words overflow-auto"
								>
									{{ debugInfo }}
									<button class="btn" @click="openDbBackupsFolder">
										<FolderOpenIcon aria-hidden="true" />
										{{ formatMessage(messages.openBackupsFolder) }}
									</button>
								</div>
								<IconButton
									v-tooltip="'复制诊断信息'"
									:label="'复制诊断信息'"
									:disabled="copied"
									@click="copyToClipboard(debugInfo)"
								>
									<template v-if="copied"> <CheckIcon class="text-green" /> </template>
									<template v-else> <CopyIcon /> </template>
								</IconButton>
							</div>
						</Collapsible>
					</div>
				</div>
			</template>
		</div>
	</ModalWrapper>
</template>

<style>
.light-mode {
	--color-orange-bg: rgba(255, 163, 71, 0.2);
}

.dark-mode,
.oled-mode {
	--color-orange-bg: rgba(224, 131, 37, 0.2);
}
</style>

<style scoped lang="scss">
.cta-button {
	display: flex;
	align-items: center;
	justify-content: center;
	padding: 0.5rem;
	gap: 0.5rem;
}

.warning-banner {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	padding: var(--gap-lg);
	background-color: var(--color-orange-bg);
	border: 2px solid var(--color-orange);
	border-radius: var(--radius-md);
	margin-bottom: 1rem;
}

.warning-banner__title {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	font-weight: 700;

	svg {
		color: var(--color-orange);
		height: 1.5rem;
		width: 1.5rem;
	}
}

.modal-body {
	display: flex;
	flex-direction: column;
	gap: var(--gap-md);
}

.markdown-body {
	overflow: auto;
}
</style>
