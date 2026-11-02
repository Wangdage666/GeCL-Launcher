<template>
	<NewModal ref="modal" header="还原备份" fade="danger" width="500px">
		<div class="flex flex-col gap-6">
			<Admonition v-if="ctx.isServerRunning.value" type="critical" header="服务器正在运行"> 请先停止服务器，再还原备份。 </Admonition>
			<Admonition v-else type="critical" header="将替换服务器文件"> 还原备份会替换当前世界及服务器文件。备份之后的更改将永久丢失。 </Admonition>

			<div v-if="currentBackup" class="flex flex-col gap-2">
				<span class="font-semibold text-contrast">Backup</span>
				<BackupItem :backup="currentBackup" preview class="!bg-surface-2 !shadow-none" />
			</div>
		</div>

		<template #actions>
			<div class="flex gap-2 justify-end">
				<Button type="outlined" @click="modal?.hide()">
					<XIcon /> 取消 </Button>
				<Button
					v-tooltip="restoreDisabledTooltip"
					type="colored"
					color="red"
					:disabled="restoreDisabled"
					@click="restoreBackup"
				>
					<SpinnerIcon v-if="isRestoring" class="animate-spin" />
					<RotateCounterClockwiseIcon v-else />
					{{ isRestoring ? 'Restoring...' : 'Restore backup' }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import type { Archon } from '@modrinth/api-client'
import { RotateCounterClockwiseIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { computed, ref } from 'vue'

import { Button } from '#ui/components/base/buttons'

import { useVIntl } from '../../../composables/i18n'
import {
	injectModrinthClient,
	injectModrinthServerContext,
	injectNotificationManager,
} from '../../../providers'
import { commonMessages } from '../../../utils'
import Admonition from '../../base/Admonition.vue'
import NewModal from '../../modal/NewModal.vue'
import BackupItem from './BackupItem.vue'

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()
const client = injectModrinthClient()
const queryClient = useQueryClient()
const ctx = injectModrinthServerContext()

const props = withDefaults(
	defineProps<{
		canRestore?: boolean
		permissionDeniedMessage?: string
	}>(),
	{
		canRestore: true,
		permissionDeniedMessage: undefined,
	},
)

const backupsQueryKey = ['backups', 'queue', ctx.serverId]

function safetyBackupName(backupName: string) {
	const base = `Before restoring "${backupName}"`
	return base.slice(0, 92)
}

const restoreMutation = useMutation({
	mutationFn: ({ backupId, name }: { backupId: string; name: string }) =>
		client.archon.backups_queue_v1.restore(ctx.serverId, ctx.worldId.value!, backupId, { name }),
	onSuccess: () => queryClient.invalidateQueries({ queryKey: backupsQueryKey }),
})

const modal = ref<InstanceType<typeof NewModal>>()
const currentBackup = ref<Archon.BackupsQueue.v1.BackupQueueBackup | null>(null)
const isRestoring = ref(false)
const restoreDisabled = computed(
	() => isRestoring.value || ctx.isServerRunning.value || !props.canRestore,
)
const restoreDisabledTooltip = computed(() =>
	props.canRestore
		? undefined
		: (props.permissionDeniedMessage ?? formatMessage(commonMessages.noPermissionAction)),
)

function show(backup: Archon.BackupsQueue.v1.BackupQueueBackup) {
	currentBackup.value = backup
	modal.value?.show()
}

const restoreBackup = () => {
	if (!props.canRestore || !currentBackup.value || isRestoring.value) {
		if (!currentBackup.value) {
			addNotification({
				type: 'error',
				title: "还原备份失败",
				text: "未选择备份",
			})
		}
		return
	}

	isRestoring.value = true
	restoreMutation.mutate(
		{
			backupId: currentBackup.value.id,
			name: safetyBackupName(currentBackup.value.name),
		},
		{
			onSuccess: () => {
				modal.value?.hide()
			},
			onError: (error) => {
				const message = error instanceof Error ? error.message : String(error)
				addNotification({ type: 'error', title: "还原备份失败", text: message })
			},
			onSettled: () => {
				isRestoring.value = false
			},
		},
	)
}

defineExpose({
	show,
})
</script>
