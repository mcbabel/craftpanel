<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.createBackup)"
		width="500px"
		@show="focusInput"
	>
		<div class="flex flex-col gap-2 -mb-2">
			<label for="backup-name-input">
				<span class="text-lg font-semibold text-contrast">{{ formatMessage(messages.nameLabel) }}</span>
			</label>
			<Input
				id="backup-name-input"
				ref="input"
				v-model="backupName"
				:placeholder="formatMessage(messages.defaultName, { number: newBackupAmount })"
				:maxlength="48"
				wrapper-class="w-full"
			/>
			<Transition
				enter-active-class="transition-all duration-300 ease-out"
				enter-from-class="opacity-0 max-h-0"
				enter-to-class="opacity-100 max-h-20"
				leave-active-class="transition-all duration-200 ease-in"
				leave-from-class="opacity-100 max-h-20"
				leave-to-class="opacity-0 max-h-0"
			>
				<div
					v-if="nameExists && !createMutation.isPending.value"
					class="flex items-center gap-1 mt-2 overflow-hidden"
				>
					<IssuesIcon class="hidden text-orange sm:block" />
					<span class="text-sm text-orange">
						{{ formatMessage(messages.nameExistsPrefix)
						}}<span class="font-semibold">{{ trimmedName }}</span
						>{{ formatMessage(messages.nameExistsSuffix) }}
					</span>
				</div>
			</Transition>
			<Transition
				enter-active-class="transition-all duration-300 ease-out"
				enter-from-class="opacity-0 max-h-0"
				enter-to-class="opacity-100 max-h-20"
				leave-active-class="transition-all duration-200 ease-in"
				leave-from-class="opacity-100 max-h-20"
				leave-to-class="opacity-0 max-h-0"
			>
				<div v-if="isRateLimited" class="overflow-hidden text-sm text-red">
					{{ formatMessage(messages.rateLimited) }}
				</div>
			</Transition>
		</div>
		<template #actions>
			<div class="flex gap-2 justify-end">
				<Button type="outlined" @click="hideModal">
					<XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button
					v-tooltip="createDisabledTooltip"
					type="colored"
					color="brand"
					:disabled="createDisabled"
					@click="createBackup"
				>
					<PlusIcon />
					{{ formatMessage(messages.createBackup) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import type { Archon } from '@modrinth/api-client'
import { IssuesIcon, PlusIcon, XIcon } from '@modrinth/assets'
import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { computed, nextTick, ref } from 'vue'

import { Button } from '#ui/components/base/buttons'

import { defineMessages, useVIntl } from '../../../composables/i18n'
import {
	injectModrinthClient,
	injectModrinthServerContext,
	injectNotificationManager,
} from '../../../providers'
import { commonMessages } from '../../../utils'
import Input from '../../base/inputs/Input.vue'
import NewModal from '../../modal/NewModal.vue'

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	createBackup: {
		id: 'craftpanel.vendor.backups.create-backup',
		defaultMessage: 'Create backup',
	},
	nameLabel: {
		id: 'craftpanel.vendor.backups.name-label',
		defaultMessage: 'Name',
	},
	defaultName: {
		id: 'craftpanel.vendor.backups.default-name',
		defaultMessage: 'Backup #{number}',
	},
	nameExistsPrefix: {
		id: 'craftpanel.vendor.backups.name-exists-prefix',
		defaultMessage: "You already have a backup named '",
	},
	nameExistsSuffix: {
		id: 'craftpanel.vendor.backups.name-exists-suffix',
		defaultMessage: "'",
	},
	rateLimited: {
		id: 'craftpanel.vendor.backups.rate-limited',
		defaultMessage: "You're creating backups too fast. Please wait a moment before trying again.",
	},
	errorCreating: {
		id: 'craftpanel.vendor.backups.error-creating',
		defaultMessage: 'Error creating backup',
	},
	tooFast: {
		id: 'craftpanel.vendor.backups.too-fast',
		defaultMessage: "You're creating backups too fast.",
	},
})
const client = injectModrinthClient()
const queryClient = useQueryClient()
const ctx = injectModrinthServerContext()

const props = withDefaults(
	defineProps<{
		backups?: Archon.BackupsQueue.v1.BackupQueueBackup[]
		canCreate?: boolean
		permissionDeniedMessage?: string
	}>(),
	{
		backups: undefined,
		canCreate: true,
		permissionDeniedMessage: undefined,
	},
)

const backupsQueryKey = ['backups', 'queue', ctx.serverId]

const createMutation = useMutation({
	mutationFn: (name: string) =>
		client.archon.backups_queue_v1.create(ctx.serverId, ctx.worldId.value!, { name }),
	onSuccess: () => queryClient.invalidateQueries({ queryKey: backupsQueryKey }),
})

const modal = ref<InstanceType<typeof NewModal>>()
const input = ref<HTMLInputElement>()
const isRateLimited = ref(false)
const backupName = ref('')
const newBackupAmount = computed(() => (props.backups?.length ?? 0) + 1)

const trimmedName = computed(() => backupName.value.trim())

const nameExists = computed(() => {
	if (!props.backups) return false
	return props.backups.some(
		(backup) => backup.name.trim().toLowerCase() === trimmedName.value.toLowerCase(),
	)
})
const createDisabled = computed(
	() => createMutation.isPending.value || nameExists.value || !props.canCreate,
)
const createDisabledTooltip = computed(() =>
	props.canCreate
		? undefined
		: (props.permissionDeniedMessage ?? formatMessage(commonMessages.noPermissionAction)),
)

const focusInput = () => {
	nextTick(() => {
		setTimeout(() => {
			input.value?.focus()
		}, 100)
	})
}

function show() {
	backupName.value = ''
	isRateLimited.value = false
	modal.value?.show()
}

const hideModal = () => {
	modal.value?.hide()
}

const createBackup = () => {
	if (!props.canCreate) return
	const name =
		trimmedName.value || formatMessage(messages.defaultName, { number: newBackupAmount.value })
	isRateLimited.value = false

	createMutation.mutate(name, {
		onSuccess: () => {
			hideModal()
		},
		onError: (error) => {
			if (error instanceof Error && error.message.includes('429')) {
				isRateLimited.value = true
				addNotification({
					type: 'error',
					title: formatMessage(messages.errorCreating),
					text: formatMessage(messages.tooFast),
				})
			} else {
				const message = error instanceof Error ? error.message : String(error)
				addNotification({
					type: 'error',
					title: formatMessage(messages.errorCreating),
					text: message,
				})
			}
		},
	})
}

defineExpose({
	show,
	hide: hideModal,
})
</script>
