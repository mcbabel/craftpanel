import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'

import { useServerBackupsQueue } from '#ui/composables/server-backups-queue'
import {
	injectAppBackup,
	injectModrinthClient,
	injectModrinthServerContext,
	injectNotificationManager,
} from '#ui/providers'

import { defineMessages, useVIntl } from '../../../../composables/i18n'

const messages = defineMessages({
	errorCreating: {
		id: 'craftpanel.vendor.inline-backup.error-creating',
		defaultMessage: 'Error creating backup',
	},
	tooFast: {
		id: 'craftpanel.vendor.inline-backup.too-fast',
		defaultMessage: "You're creating backups too fast.",
	},
	cancelledTitle: {
		id: 'craftpanel.vendor.inline-backup.cancelled-title',
		defaultMessage: 'Backup cancelled',
	},
	cancelledText: {
		id: 'craftpanel.vendor.inline-backup.cancelled-text',
		defaultMessage:
			'The backup has been cancelled. You can create a new one or proceed without a backup.',
	},
	leaveConfirm: {
		id: 'craftpanel.vendor.inline-backup.leave-confirm',
		defaultMessage: 'A backup is being created. Are you sure you want to leave?',
	},
})

export function useInlineBackup(backupName: string | (() => string)) {
	const serverCtx = injectModrinthServerContext(null)
	const appBackup = injectAppBackup(null)

	if (!serverCtx) {
		if (appBackup) {
			const isBackingUp = ref(false)
			const backupFailed = ref(false)
			const backupComplete = ref(false)

			return {
				available: true as const,
				isServer: false as const,
				isBackingUp,
				isCancelling: ref(false),
				backupFailed,
				backupComplete,
				backupCancelled: ref(false),
				externalBackupInProgress: computed(() => false),
				startBackup: async () => {
					isBackingUp.value = true
					backupFailed.value = false
					backupComplete.value = false
					try {
						await appBackup.createBackup()
						backupComplete.value = true
					} catch {
						backupFailed.value = true
					} finally {
						isBackingUp.value = false
					}
				},
				cancelBackup: async () => {},
			}
		}

		return {
			available: false as const,
			isServer: false as const,
			isBackingUp: ref(false),
			isCancelling: ref(false),
			backupFailed: ref(false),
			backupComplete: ref(false),
			backupCancelled: ref(false),
			externalBackupInProgress: ref(false),
			startBackup: async () => {},
			cancelBackup: async () => {},
		}
	}

	const client = injectModrinthClient()
	const { addNotification } = injectNotificationManager()
	const { formatMessage } = useVIntl()
	const { serverId, worldId } = serverCtx

	const { activeOperationByBackupId, backups, hasActiveCreate, invalidate } = useServerBackupsQueue(
		computed(() => serverId),
		worldId,
	)

	const createdBackupId = ref<string | null>(null)
	const createdOperationId = ref<number | null>(null)
	const pendingCreate = ref(false)
	const backupFailed = ref(false)
	const backupComplete = ref(false)
	const backupCancelled = ref(false)
	const isCancelling = ref(false)

	const myBackup = computed(() =>
		createdBackupId.value ? backups.value.find((b) => b.id === createdBackupId.value) : undefined,
	)
	const myActiveOp = computed(() =>
		createdBackupId.value ? activeOperationByBackupId.value.get(createdBackupId.value) : undefined,
	)

	const isBackingUp = computed(
		() =>
			!backupComplete.value &&
			!backupFailed.value &&
			!backupCancelled.value &&
			(!!createdBackupId.value || pendingCreate.value),
	)

	const externalBackupInProgress = computed(() => hasActiveCreate.value && !myActiveOp.value)

	watch(
		myBackup,
		(b) => {
			if (!createdBackupId.value || !b) return
			if (b.status === 'done') backupComplete.value = true
			else if (b.status === 'error' || b.status === 'timed_out') backupFailed.value = true
		},
		{ immediate: true },
	)

	watch(
		myActiveOp,
		(op) => {
			if (op?.operation_id != null) {
				createdOperationId.value = op.operation_id
			}
		},
		{ immediate: true },
	)

	async function startBackup() {
		if (!worldId.value) return

		const name = typeof backupName === 'function' ? backupName() : backupName

		backupFailed.value = false
		backupComplete.value = false
		backupCancelled.value = false
		isCancelling.value = false
		createdBackupId.value = null
		createdOperationId.value = null
		pendingCreate.value = true

		try {
			const { id } = await client.archon.backups_queue_v1.create(serverId, worldId.value, { name })
			createdBackupId.value = id
			await invalidate()
		} catch (error) {
			backupFailed.value = true
			const message = error instanceof Error ? error.message : String(error)
			const isRateLimit = message.includes('429')
			addNotification({
				type: 'error',
				title: formatMessage(messages.errorCreating),
				text: isRateLimit ? formatMessage(messages.tooFast) : message,
			})
		} finally {
			pendingCreate.value = false
		}
	}

	async function cancelBackup() {
		if (!worldId.value || !createdBackupId.value || !isBackingUp.value) return

		isCancelling.value = true
		try {
			let operationId = createdOperationId.value ?? myActiveOp.value?.operation_id ?? null
			if (operationId == null) {
				const queue = await client.archon.backups_queue_v1.list(serverId, worldId.value)
				const operation = queue.active_operations.find(
					(op) => op.backup_id === createdBackupId.value && op.operation_type === 'create',
				)
				operationId = operation?.operation_id ?? null
			}
			if (operationId == null) {
				throw new Error('Could not find the backup creation operation to cancel.')
			}
			await client.archon.backups_queue_v1.cancelCreate(serverId, worldId.value, operationId)
			backupCancelled.value = true
			isCancelling.value = false
			await invalidate()
			addNotification({
				type: 'info',
				title: formatMessage(messages.cancelledTitle),
				text: formatMessage(messages.cancelledText),
			})
		} catch {
			backupFailed.value = true
			isCancelling.value = false
		}
	}

	function handleBeforeUnload(e: BeforeUnloadEvent) {
		if (isBackingUp.value) {
			e.preventDefault()
			return ''
		}
	}

	if (typeof window !== 'undefined') {
		watch(isBackingUp, (operating) => {
			if (operating) {
				window.addEventListener('beforeunload', handleBeforeUnload)
			} else {
				window.removeEventListener('beforeunload', handleBeforeUnload)
			}
		})

		onBeforeUnmount(() => {
			window.removeEventListener('beforeunload', handleBeforeUnload)
		})

		onBeforeRouteLeave(() => {
			if (isBackingUp.value) {
				return window.confirm(formatMessage(messages.leaveConfirm))
			}
			return true
		})
	}

	return {
		available: true as const,
		isServer: true as const,
		isBackingUp,
		isCancelling,
		backupFailed,
		backupComplete,
		backupCancelled,
		externalBackupInProgress,
		startBackup,
		cancelBackup,
	}
}
