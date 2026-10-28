<template>
	<div class="flex flex-col gap-8">
		<div class="flex flex-wrap items-start justify-start gap-x-16 gap-y-8">
			<div class="flex min-w-0 max-w-[32rem] grow basis-64 flex-col gap-2.5">
				<label for="server-name-field" class="text-lg font-semibold text-contrast">
					{{ formatMessage(messages.nameLabel) }}
				</label>
				<Input
					id="server-name-field"
					v-model="name"
					v-tooltip="renameTooltip"
					wrapper-class="w-full"
					:maxlength="48"
					:disabled="!canUseAdvancedSettings"
				/>
				<span v-if="!isValidName" class="font-medium text-red">
					{{ formatMessage(messages.nameEmpty) }}
				</span>
			</div>

			<EditServerIcon
				:can-edit="canUseAdvancedSettings"
				:permission-denied-message="permissionDeniedMessage"
			/>
		</div>

		<div class="flex flex-col gap-2.5">
			<span class="text-lg font-semibold text-contrast">
				{{ formatMessage(messages.updateChannel) }}
			</span>
			<Chips
				v-model="channel"
				:items="CHANNELS"
				:disabled-items="channelDisabledItems"
				:disabled-tooltip="renameTooltip"
				:aria-label="formatMessage(messages.updateChannel)"
			/>
			<span>{{ formatMessage(CHANNEL_DESCRIPTIONS[channel]) }}</span>
		</div>

		<div class="flex flex-col gap-2.5">
			<span class="text-lg font-semibold text-contrast">{{ formatMessage(messages.info) }}</span>
			<div class="flex flex-col gap-2.5 rounded-xl bg-surface-2 p-4">
				<div v-for="entry in info" :key="entry.label" class="flex items-start justify-between gap-4">
					<span class="mt-1">{{ entry.label }}</span>
					<CopyCode v-if="entry.copy" :text="entry.value" />
					<span v-else class="text-right text-sm break-words">{{ entry.value }}</span>
				</div>
			</div>
		</div>

		<div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
			<div class="flex flex-col gap-1">
				<span class="text-lg font-semibold text-contrast">
					{{ formatMessage(messages.downloadAll) }}
				</span>
				<span>{{ formatMessage(messages.downloadAllDescription) }}</span>
			</div>
			<Button
				v-tooltip="downloadTooltip"
				:disabled="downloadDisabled"
				@click="downloadEverything"
			>
				<DownloadIcon class="size-5" />
				{{ formatMessage(packing ? messages.downloadPreparing : messages.downloadAllButton) }}
			</Button>
		</div>

		<div class="flex flex-col gap-4 rounded-2xl border-2 border-solid border-red p-4">
			<div class="flex flex-col gap-1">
				<span class="text-lg font-semibold text-red">{{ formatMessage(messages.danger) }}</span>
				<span>{{ formatMessage(messages.dangerDescription) }}</span>
			</div>

			<BackupWarning :backup-link="backupLink" />

			<div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
				<div class="flex flex-col gap-1">
					<span class="font-semibold text-contrast">{{ formatMessage(messages.reset) }}</span>
					<span>{{ formatMessage(messages.resetDescription) }}</span>
				</div>
				<Button
					v-tooltip="resetTooltip"
					type="colored"
					color="red"
					:disabled="resetDisabled"
					@click="resetModal?.show()"
				>
					<RotateCounterClockwiseIcon class="size-5" />
					{{ formatMessage(messages.reset) }}
				</Button>
			</div>

			<div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
				<div class="flex flex-col gap-1">
					<span class="font-semibold text-contrast">{{ formatMessage(messages.remove) }}</span>
					<span>{{ formatMessage(messages.removeDescription) }}</span>
				</div>
				<Button
					v-tooltip="removeTooltip"
					type="colored"
					color="red"
					:disabled="removeDisabled"
					@click="removeModal?.show()"
				>
					<TrashIcon class="size-5" />
					{{ formatMessage(messages.remove) }}
				</Button>
			</div>
		</div>

		<Teleport to="body">
			<div class="relative z-[100]">
				<ConfirmModal
					ref="resetModal"
					:title="formatMessage(messages.reset)"
					:description="formatMessage(messages.resetConfirm, { name: serverName })"
					:proceed-label="formatMessage(messages.reset)"
					:confirmation-text="serverName"
					has-to-type
					:proceed-icon="RotateCounterClockwiseIcon"
					@proceed="resetServer"
				/>
				<ConfirmModal
					ref="removeModal"
					:title="formatMessage(messages.remove)"
					:description="formatMessage(messages.removeConfirm, { name: serverName })"
					:proceed-label="formatMessage(messages.remove)"
					:confirmation-text="serverName"
					has-to-type
					@proceed="removeServer"
				/>
			</div>
		</Teleport>

		<SaveBanner
			:is-visible="hasChanges && isValidName"
			:server-id="serverId"
			:is-updating="saving"
			:save="save"
			:reset="reset"
		/>
	</div>
</template>

<script setup lang="ts">
import { DownloadIcon, RotateCounterClockwiseIcon, TrashIcon } from '@modrinth/assets'
import {
	BackupWarning,
	Button,
	Chips,
	ConfirmModal,
	CopyCode,
	defineMessages,
	injectModrinthServerContext,
	injectNotificationManager,
	Input,
	SaveBanner,
	useServerPermissions,
	useVIntl,
} from '@modrinth/ui'
import EditServerIcon from '@modrinth/ui/src/components/servers/edit-server-icon/EditServerIcon.vue'
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import { api, type LoaderId, type UpdateChannel } from '@/api'
import { useServerPage } from '@/composables/server-page'
import { useSession } from '@/composables/session'
import { startDownload } from '@/providers/file-manager'

const LOADERS: LoaderId[] = [
	'vanilla',
	'paper',
	'folia',
	'purpur',
	'leaf',
	'fabric',
	'velocity',
	'neoforge',
	'quilt',
	'forge',
]

const CHANNELS: UpdateChannel[] = ['release', 'beta', 'alpha']

const { formatMessage } = useVIntl()
const router = useRouter()
const { isAdmin, user } = useSession()
const { addNotification } = injectNotificationManager()
const { server, serverId, busyReasons, powerState } = injectModrinthServerContext()
const { server: panelServer, operations } = useServerPage()
const { canUseAdvancedSettings, canResetServer, canWriteFiles, permissionDeniedMessage } =
	useServerPermissions()

const messages = defineMessages({
	nameLabel: { id: 'craftpanel.settings.general.name', defaultMessage: 'Server name' },
	nameEmpty: {
		id: 'craftpanel.settings.general.name-empty',
		defaultMessage: 'Server name cannot be empty.',
	},
	updateChannel: {
		id: 'craftpanel.settings.general.update-channel',
		defaultMessage: 'Update channel',
	},
	channelRelease: {
		id: 'craftpanel.settings.general.update-channel.release',
		defaultMessage: 'Only release versions will be shown as available updates.',
	},
	channelBeta: {
		id: 'craftpanel.settings.general.update-channel.beta',
		defaultMessage: 'Release and beta versions will be shown as available updates.',
	},
	channelAlpha: {
		id: 'craftpanel.settings.general.update-channel.alpha',
		defaultMessage: 'Release, beta, and alpha versions will be shown as available updates.',
	},
	info: { id: 'craftpanel.settings.general.info', defaultMessage: 'Info' },
	serverId: { id: 'craftpanel.settings.general.server-id', defaultMessage: 'Server ID' },
	address: { id: 'craftpanel.settings.general.address', defaultMessage: 'Address' },
	version: { id: 'craftpanel.settings.general.version', defaultMessage: 'Version' },
	backups: { id: 'craftpanel.settings.general.backups', defaultMessage: 'Backups' },
	danger: { id: 'craftpanel.settings.general.danger', defaultMessage: 'Danger zone' },
	dangerDescription: {
		id: 'craftpanel.settings.general.danger-description',
		defaultMessage: 'These actions destroy data. The server has to be stopped for all of them.',
	},
	reset: { id: 'craftpanel.settings.general.reset', defaultMessage: 'Reset server' },
	resetDescription: {
		id: 'craftpanel.settings.general.reset-description',
		defaultMessage:
			'Deletes world, content and configuration and installs the same version again. Backups remain.',
	},
	resetConfirm: {
		id: 'craftpanel.settings.general.reset-confirm',
		defaultMessage: 'Everything on {name} except its backups will be deleted and reinstalled.',
	},
	remove: { id: 'craftpanel.settings.general.remove', defaultMessage: 'Delete server' },
	removeDescription: {
		id: 'craftpanel.settings.general.remove-description',
		defaultMessage: 'Removes the server, its files and its backups. This cannot be undone.',
	},
	removeConfirm: {
		id: 'craftpanel.settings.general.remove-confirm',
		defaultMessage: '{name} and everything belonging to it will be deleted.',
	},
	saved: { id: 'craftpanel.settings.general.saved', defaultMessage: 'Server settings updated' },
	saveFailed: {
		id: 'craftpanel.settings.general.save-failed',
		defaultMessage: 'Failed to update the server',
	},
	resetStarted: {
		id: 'craftpanel.settings.general.reset-started',
		defaultMessage: 'The server is being reset',
	},
	resetFailed: { id: 'craftpanel.settings.general.reset-failed', defaultMessage: 'Failed to reset' },
	removeStarted: {
		id: 'craftpanel.settings.general.remove-started',
		defaultMessage: 'The server is being deleted',
	},
	removeFailed: { id: 'craftpanel.settings.general.remove-failed', defaultMessage: 'Failed to delete' },
	running: {
		id: 'craftpanel.settings.general.running',
		defaultMessage: 'Stop the server first.',
	},
	busy: {
		id: 'craftpanel.settings.general.busy',
		defaultMessage: 'Wait until the running operation is done.',
	},
	notInstalled: {
		id: 'craftpanel.settings.general.not-installed',
		defaultMessage: 'This server has no installation yet.',
	},
	notOwner: {
		id: 'craftpanel.settings.general.not-owner',
		defaultMessage: 'Only the owner of the server can do this.',
	},
	unknown: { id: 'craftpanel.settings.general.unknown', defaultMessage: 'Unknown' },
	downloadAll: {
		id: 'craftpanel.settings.general.download-all',
		defaultMessage: 'Download server files',
	},
	downloadAllDescription: {
		id: 'craftpanel.settings.general.download-all-description',
		defaultMessage:
			'Packs the world and every other file of this server into a ZIP in its main folder and downloads it once it is ready. The ZIP stays there until you delete it.',
	},
	downloadAllButton: {
		id: 'craftpanel.settings.general.download-all-button',
		defaultMessage: 'Download as ZIP',
	},
	downloadPreparing: {
		id: 'craftpanel.settings.general.download-preparing',
		defaultMessage: 'Creating ZIP…',
	},
	downloadStarted: {
		id: 'craftpanel.settings.general.download-started',
		defaultMessage: 'Creating the ZIP. The download starts when it is ready.',
	},
	downloadFailed: {
		id: 'craftpanel.settings.general.download-failed',
		defaultMessage: 'Could not create the ZIP',
	},
	downloadEmpty: {
		id: 'craftpanel.settings.general.download-empty',
		defaultMessage: 'This server has no files yet.',
	},
})

const CHANNEL_DESCRIPTIONS: Record<UpdateChannel, (typeof messages)[keyof typeof messages]> = {
	release: messages.channelRelease,
	beta: messages.channelBeta,
	alpha: messages.channelAlpha,
}

const serverName = computed(() => server.value?.name ?? '')
const savedChannel = computed(() => panelServer.value.update_channel)
const name = ref(serverName.value)
const channel = ref<UpdateChannel>(savedChannel.value)
const saving = ref(false)
const resetModal = ref<InstanceType<typeof ConfirmModal>>()
const removeModal = ref<InstanceType<typeof ConfirmModal>>()

watch(serverName, (value) => {
	if (!saving.value) name.value = value
})

watch(savedChannel, (value) => {
	if (!saving.value) channel.value = value
})

const isValidName = computed(() => name.value.trim().length > 0)
const hasChanges = computed(
	() => name.value !== serverName.value || channel.value !== savedChannel.value,
)
const renameTooltip = computed(() =>
	canUseAdvancedSettings.value ? undefined : permissionDeniedMessage.value,
)
const channelDisabledItems = computed<UpdateChannel[]>(() =>
	canUseAdvancedSettings.value && !saving.value ? [] : [...CHANNELS],
)

const busy = computed(() => busyReasons.value.length > 0)
const isRunning = computed(() => powerState.value !== 'stopped' && powerState.value !== 'crashed')
const isOwner = computed(() => server.value?.owner_id === user.value?.id)
const backupLink = computed(() => `/servers/${serverId}/backups`)

const loaderId = computed<LoaderId | null>(() => {
	const name = server.value?.loader?.toLowerCase()
	return LOADERS.find((loader) => loader === name) ?? null
})

const resetDisabled = computed(
	() => !canResetServer.value || isRunning.value || busy.value || loaderId.value === null,
)
const removeDisabled = computed(
	() => (!isOwner.value && !isAdmin.value) || isRunning.value || busy.value,
)

function guardTooltip(allowed: boolean): string | undefined {
	if (!allowed) return permissionDeniedMessage.value
	if (isRunning.value) return formatMessage(messages.running)
	if (busy.value) return formatMessage(messages.busy)
	return undefined
}

const resetTooltip = computed(() => {
	if (canResetServer.value && loaderId.value === null) return formatMessage(messages.notInstalled)
	return guardTooltip(canResetServer.value)
})
const removeTooltip = computed(() =>
	isOwner.value || isAdmin.value ? guardTooltip(true) : formatMessage(messages.notOwner),
)

const packing = ref<string | null>(null)
const downloadDisabled = computed(() => !canWriteFiles.value || packing.value !== null || busy.value)
const downloadTooltip = computed(() => {
	if (!canWriteFiles.value) return permissionDeniedMessage.value
	if (busy.value) return formatMessage(messages.busy)
	return undefined
})

function archiveStem(): string {
	const plain = serverName.value.replace(/[^A-Za-z0-9 ._-]/g, '_').trim().slice(0, 100)
	return plain.replace(/^\.+/, '') || 'server'
}

async function downloadEverything(): Promise<void> {
	if (downloadDisabled.value) return
	packing.value = ''
	try {
		const stem = archiveStem()
		const quoted = stem.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
		const earlier = new RegExp(`^${quoted}( \\(\\d+\\))?\\.zip$`, 'i')
		const { items } = await api.files.listAll(serverId, '/')
		const include = items.map((item) => item.name).filter((name) => !earlier.test(name))
		if (include.length === 0) {
			packing.value = null
			addNotification({ type: 'info', title: formatMessage(messages.downloadEmpty) })
			return
		}
		const { operation } = await api.files.zip(serverId, {
			parent: '/',
			include,
			target: `${stem}.zip`,
			on_conflict: 'rename',
		})
		packing.value = operation.id
		addNotification({ type: 'info', title: formatMessage(messages.downloadStarted) })
		checkPacking()
	} catch (error) {
		packing.value = null
		report(messages.downloadFailed, error)
	}
}

function checkPacking(): void {
	const id = packing.value
	if (!id) return
	const operation = operations.value.find((candidate) => candidate.id === id)
	if (!operation || !['done', 'failed', 'cancelled'].includes(operation.state)) return
	packing.value = null
	if (operation.state !== 'done' || !operation.src) return
	const name = operation.src.split('/').pop() ?? 'server.zip'
	startDownload(api.files.contentUrl(serverId, { path: operation.src, download: 1 }), name)
}

watch(operations, checkPacking)

const info = computed(() => {
	const net = server.value?.net
	const version = [server.value?.loader, server.value?.mc_version].filter(Boolean).join(' ')
	return [
		{ label: formatMessage(messages.serverId), value: serverId, copy: true },
		{
			label: formatMessage(messages.address),
			value: net?.ip ? `${net.ip}:${net.port}` : String(net?.port ?? ''),
			copy: true,
		},
		{
			label: formatMessage(messages.version),
			value: version || formatMessage(messages.unknown),
			copy: false,
		},
		{
			label: formatMessage(messages.backups),
			value: `${server.value?.used_backup_quota ?? 0} / ${server.value?.backup_quota ?? 0}`,
			copy: false,
		},
	]
})

function report(descriptor: (typeof messages)[keyof typeof messages], cause: unknown): void {
	addNotification({
		type: 'error',
		title: formatMessage(descriptor),
		text: cause instanceof Error ? cause.message : undefined,
	})
}

async function save(): Promise<void> {
	if (!canUseAdvancedSettings.value || !isValidName.value) return
	saving.value = true
	try {
		await api.servers.update(serverId, {
			name: name.value.trim(),
			update_channel: channel.value,
		})
		addNotification({ type: 'success', title: formatMessage(messages.saved) })
	} catch (error) {
		report(messages.saveFailed, error)
	} finally {
		saving.value = false
	}
}

function reset(): void {
	name.value = serverName.value
	channel.value = savedChannel.value
}

async function resetServer(): Promise<void> {
	const loader = loaderId.value
	const gameVersion = server.value?.mc_version
	if (loader === null || !gameVersion) return
	try {
		await api.settings.reset(serverId, {
			loader,
			game_version: gameVersion,
			loader_version: server.value?.loader_version ?? null,
			keep_backups: true,
		})
		addNotification({ type: 'success', title: formatMessage(messages.resetStarted) })
	} catch (error) {
		report(messages.resetFailed, error)
	}
}

async function removeServer(): Promise<void> {
	try {
		await api.servers.remove(serverId)
		addNotification({ type: 'success', title: formatMessage(messages.removeStarted) })
		await router.push({ name: 'servers' })
	} catch (error) {
		report(messages.removeFailed, error)
	}
}
</script>
