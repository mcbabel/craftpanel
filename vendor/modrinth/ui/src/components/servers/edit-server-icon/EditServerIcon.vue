<template>
	<IconEditorModal
		ref="iconEditorModal"
		:config="generatedConfig"
		:load-recents="loadRecentConfigs"
		:save="saveGeneratedIcon"
	/>
	<div class="flex flex-col gap-2.5">
		<span class="text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</span>
		<div class="group relative w-fit">
			<TeleportOverflowMenu
				:label="editIconTooltip"
				:tooltip="editIconTooltip"
				:icon-only="false"
				type="quiet"
				interaction="none"
				class="m-0 !h-auto cursor-pointer appearance-none border-none bg-transparent !p-0 transition-transform group-active:scale-95"
				:disabled="isIconActionDisabled"
				:options="[
					{
						id: 'upload',
						label: formatMessage(messages.upload),
						action: () => triggerFileInput(),
						disabled: !props.canEdit,
						tooltip: !props.canEdit ? editIconTooltip : undefined,
					},
					{
						id: 'create',
						label: formatMessage(generatedConfig ? messages.editCreated : messages.create),
						action: () => openIconEditor(),
						disabled: !props.canEdit,
						tooltip: !props.canEdit ? editIconTooltip : undefined,
					},
					{
						id: 'sync',
						label: formatMessage(messages.sync),
						action: () => resetIcon(),
						disabled: !props.canEdit,
						tooltip: !props.canEdit ? editIconTooltip : undefined,
					},
				]"
			>
				<ServerIcon
					class="size-28 transition-[filter] group-hover:brightness-[0.50]"
					:class="isIconActionLoading ? 'brightness-[0.50]' : ''"
					:image="displayIcon"
				/>
				<div
					class="absolute top-0 h-full w-full flex items-center justify-center"
					:class="isIconActionLoading ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'"
				>
					<SpinnerIcon
						v-if="isIconActionLoading"
						aria-hidden="true"
						class="h-10 w-10 animate-spin text-primary"
					/>
					<EditIcon v-else aria-hidden="true" class="h-10 w-10 text-primary" />
				</div>
				<template #upload> <UploadIcon /> {{ formatMessage(messages.upload) }} </template>
				<template #create>
					<PaletteIcon />
					{{ formatMessage(generatedConfig ? messages.editCreated : messages.create) }}
				</template>
				<template #sync> <TransferIcon /> {{ formatMessage(messages.sync) }} </template>
			</TeleportOverflowMenu>
		</div>
	</div>
</template>

<script setup lang="ts">
import { EditIcon, PaletteIcon, SpinnerIcon, TransferIcon, UploadIcon } from '@modrinth/assets'
import { useQueryClient } from '@tanstack/vue-query'
import { computed, onMounted, ref } from 'vue'

import TeleportOverflowMenu from '#ui/components/base/buttons/TeleportOverflowMenu.vue'
import { type IconConfig, renderIcon } from '#ui/components/base/icon-editor-modal'
import IconEditorModal from '#ui/components/base/icon-editor-modal/index.vue'
import ServerIcon from '#ui/components/servers/icons/ServerIcon.vue'
import { useServerImage } from '#ui/composables'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import {
	injectModrinthClient,
	injectModrinthServerContext,
	injectNotificationManager,
} from '#ui/providers'
import { commonMessages } from '#ui/utils/common-messages'

const props = withDefaults(
	defineProps<{
		canEdit?: boolean
		permissionDeniedMessage?: string
	}>(),
	{
		canEdit: true,
		permissionDeniedMessage: undefined,
	},
)

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: { id: 'craftpanel.vendor.edit-server-icon.title', defaultMessage: 'Icon' },
	upload: { id: 'craftpanel.vendor.edit-server-icon.upload', defaultMessage: 'Upload icon' },
	editCreated: {
		id: 'craftpanel.vendor.edit-server-icon.edit-created',
		defaultMessage: 'Edit created icon',
	},
	create: { id: 'craftpanel.vendor.edit-server-icon.create', defaultMessage: 'Create an icon' },
	sync: { id: 'craftpanel.vendor.edit-server-icon.sync', defaultMessage: 'Sync icon' },
	edit: { id: 'craftpanel.vendor.edit-server-icon.edit', defaultMessage: 'Edit icon' },
	unavailable: {
		id: 'craftpanel.vendor.edit-server-icon.unavailable',
		defaultMessage: 'Server icon editing is unavailable.',
	},
	noFileTitle: {
		id: 'craftpanel.vendor.edit-server-icon.no-file-title',
		defaultMessage: 'No file selected',
	},
	noFileText: {
		id: 'craftpanel.vendor.edit-server-icon.no-file-text',
		defaultMessage: 'Please select a file to upload.',
	},
	settingsNotSavedTitle: {
		id: 'craftpanel.vendor.edit-server-icon.settings-not-saved-title',
		defaultMessage: 'Icon editor settings not saved',
	},
	settingsNotSavedText: {
		id: 'craftpanel.vendor.edit-server-icon.settings-not-saved-text',
		defaultMessage: 'The server icon was updated, but its editor settings could not be saved.',
	},
	updatedTitle: {
		id: 'craftpanel.vendor.edit-server-icon.updated-title',
		defaultMessage: 'Server icon updated',
	},
	updatedText: {
		id: 'craftpanel.vendor.edit-server-icon.updated-text',
		defaultMessage: 'Your server icon was successfully changed.',
	},
	uploadFailedTitle: {
		id: 'craftpanel.vendor.edit-server-icon.upload-failed-title',
		defaultMessage: 'Upload failed',
	},
	uploadFailedText: {
		id: 'craftpanel.vendor.edit-server-icon.upload-failed-text',
		defaultMessage: 'Failed to upload server icon.',
	},
	resetTitle: {
		id: 'craftpanel.vendor.edit-server-icon.reset-title',
		defaultMessage: 'Server icon reset',
	},
	resetText: {
		id: 'craftpanel.vendor.edit-server-icon.reset-text',
		defaultMessage: 'Your server icon was successfully reset.',
	},
	resetFailedTitle: {
		id: 'craftpanel.vendor.edit-server-icon.reset-failed-title',
		defaultMessage: 'Reset failed',
	},
	resetFailedText: {
		id: 'craftpanel.vendor.edit-server-icon.reset-failed-text',
		defaultMessage: 'Failed to reset server icon.',
	},
})
const client = injectModrinthClient()
const { serverId, server } = injectModrinthServerContext()
const queryClient = useQueryClient()
const iconEditorModal = ref<InstanceType<typeof IconEditorModal> | null>(null)
const generatedConfig = ref<IconConfig | null>(null)
const isUploadingIcon = ref(false)
const isSyncingIcon = ref(false)
const isIconActionLoading = computed(() => isUploadingIcon.value || isSyncingIcon.value)
const isIconActionDisabled = computed(() => isIconActionLoading.value || !props.canEdit)
const editIconTooltip = computed(() =>
	props.canEdit
		? formatMessage(messages.edit)
		: (props.permissionDeniedMessage ?? formatMessage(commonMessages.noPermissionAction)),
)

const {
	image: displayIcon,
	refetch: refetchRemoteIcon,
	setImage,
	clearImage,
} = useServerImage(
	serverId,
	computed(() => server.value?.upstream ?? null),
	{
		includeProjectFallback: false,
	},
)

function getStatusCode(error: unknown): number | undefined {
	const err = error as { statusCode?: number; response?: { status?: number } }
	return err.statusCode ?? err.response?.status
}

function isNotFound(error: unknown): boolean {
	return getStatusCode(error) === 404
}

const configPath = '/server-icon-config.json'
const recentsKey = 'modrinth.server-icon-recents'

async function loadRecentConfigs(): Promise<IconConfig[]> {
	try {
		const value: unknown = JSON.parse(localStorage.getItem(recentsKey) ?? '[]')
		return Array.isArray(value) ? (value as IconConfig[]) : []
	} catch {
		return []
	}
}

async function saveRecentConfig(config: IconConfig) {
	try {
		const recent = await loadRecentConfigs()
		const key = JSON.stringify(config)
		localStorage.setItem(
			recentsKey,
			JSON.stringify(
				[config, ...recent.filter((entry) => JSON.stringify(entry) !== key)].slice(0, 16),
			),
		)
	} catch {
		return
	}
}

async function loadGeneratedConfig(): Promise<IconConfig | null> {
	try {
		const fsAuth = await client.archon.servers_v0.getFilesystemAuth(serverId)
		const blob = await client.kyros.files_v0.downloadFileWithAuth(fsAuth, configPath)
		const config: unknown = JSON.parse(await blob.text())
		return config &&
			typeof config === 'object' &&
			'symbol' in config &&
			typeof config.symbol === 'string' &&
			'background' in config
			? (config as IconConfig)
			: null
	} catch {
		return null
	}
}

async function saveGeneratedConfig(config: IconConfig | null) {
	const fsAuth = await client.archon.servers_v0.getFilesystemAuth(serverId)
	try {
		await client.kyros.files_v0.deleteFileOrFolderWithAuth(fsAuth, configPath, false)
	} catch (error) {
		if (!isNotFound(error)) throw error
	}
	if (config) {
		const configFile = new File([JSON.stringify(config)], 'server-icon-config.json', {
			type: 'application/json',
		})
		await client.kyros.files_v0.uploadFileWithAuth(fsAuth, configPath, configFile).promise
		await saveRecentConfig(config)
	}
	generatedConfig.value = config
}

onMounted(async () => {
	if (props.canEdit) generatedConfig.value = await loadGeneratedConfig()
})

async function openIconEditor() {
	if (isIconActionDisabled.value) return
	generatedConfig.value = await loadGeneratedConfig()
	iconEditorModal.value?.show()
}

async function saveGeneratedIcon(config: IconConfig, symbolAsset: string) {
	if (isIconActionDisabled.value) throw new Error(formatMessage(messages.unavailable))
	const icon = await renderIcon(config, symbolAsset)
	if (!(await uploadIcon(icon, config))) throw new Error(formatMessage(messages.uploadFailedText))
}

const uploadFile = async (e: Event) => {
	if (isIconActionDisabled.value) return

	const file = (e.target as HTMLInputElement).files?.[0]
	if (!file) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.noFileTitle),
			text: formatMessage(messages.noFileText),
		})
		return
	}

	await uploadIcon(file, null)
}

async function uploadIcon(file: File, config: IconConfig | null): Promise<boolean> {
	isUploadingIcon.value = true

	try {
		const scaledFile = await new Promise<File>((resolve, reject) => {
			const canvas = document.createElement('canvas')
			const ctx = canvas.getContext('2d')
			const img = new Image()
			img.onload = () => {
				canvas.width = 64
				canvas.height = 64
				ctx?.drawImage(img, 0, 0, 64, 64)
				canvas.toBlob((blob) => {
					if (blob) {
						resolve(new File([blob], 'server-icon.png', { type: 'image/png' }))
					} else {
						reject(new Error('Canvas toBlob failed'))
					}
				}, 'image/png')
				URL.revokeObjectURL(img.src)
			}
			img.onerror = reject
			img.src = URL.createObjectURL(file)
		})

		const fsAuth = await client.archon.servers_v0.getFilesystemAuth(serverId)

		try {
			await client.kyros.files_v0.uploadFileWithAuth(fsAuth, '/server-icon.png', scaledFile).promise
		} catch (scaledUploadError) {
			// Node FS may reject create when file already exists. Delete and retry once.
			try {
				await client.kyros.files_v0.deleteFileOrFolderWithAuth(fsAuth, '/server-icon.png', false)
			} catch (deleteError) {
				if (!isNotFound(deleteError)) {
					throw scaledUploadError
				}
			}

			await client.kyros.files_v0.uploadFileWithAuth(fsAuth, '/server-icon.png', scaledFile).promise
		}

		// Keep original file in sync when possible, but don't block icon updates on failures here.
		try {
			await client.kyros.files_v0.deleteFileOrFolderWithAuth(
				fsAuth,
				'/server-icon-original.png',
				false,
			)
		} catch (deleteOriginalError) {
			if (!isNotFound(deleteOriginalError)) {
				// best effort
			}
		}

		try {
			await client.kyros.files_v0.uploadFileWithAuth(fsAuth, '/server-icon-original.png', file)
				.promise
		} catch (originalUploadError) {
			if (!isNotFound(originalUploadError)) {
				// best effort
			}
		}

		const canvas = document.createElement('canvas')
		const ctx = canvas.getContext('2d')
		const img = new Image()
		await new Promise<void>((resolve) => {
			img.onload = () => {
				canvas.width = 512
				canvas.height = 512
				ctx?.drawImage(img, 0, 0, 512, 512)
				const dataURL = canvas.toDataURL('image/png')
				setImage(dataURL)
				queryClient.setQueriesData({ queryKey: ['servers', 'detail', serverId, 'icon'] }, dataURL)
				resolve()
				URL.revokeObjectURL(img.src)
			}
			img.src = URL.createObjectURL(file)
		})
		await refetchRemoteIcon()

		try {
			await saveGeneratedConfig(config)
		} catch {
			addNotification({
				type: 'error',
				title: formatMessage(messages.settingsNotSavedTitle),
				text: formatMessage(messages.settingsNotSavedText),
			})
			return true
		}

		addNotification({
			type: 'success',
			title: formatMessage(messages.updatedTitle),
			text: formatMessage(messages.updatedText),
		})
		return true
	} catch {
		addNotification({
			type: 'error',
			title: formatMessage(messages.uploadFailedTitle),
			text: formatMessage(messages.uploadFailedText),
		})
		return false
	} finally {
		isUploadingIcon.value = false
	}
}

const resetIcon = async () => {
	if (isIconActionDisabled.value) return
	isSyncingIcon.value = true

	try {
		const fsAuth = await client.archon.servers_v0.getFilesystemAuth(serverId)
		const deleteResults = await Promise.allSettled([
			client.kyros.files_v0.deleteFileOrFolderWithAuth(fsAuth, '/server-icon.png', false),
			client.kyros.files_v0.deleteFileOrFolderWithAuth(fsAuth, '/server-icon-original.png', false),
			client.kyros.files_v0.deleteFileOrFolderWithAuth(fsAuth, configPath, false),
		])

		for (const result of deleteResults) {
			if (result.status === 'rejected' && !isNotFound(result.reason)) {
				throw result.reason
			}
		}

		// Force default icon state across all useServerImage instances via the shared query cache.
		// Use `null` (not `undefined`) because TanStack Query v5 treats setQueriesData(undefined)
		// as a no-op. The `null` sentinel is handled by useServerImage's image computed.
		clearImage()
		generatedConfig.value = null
		await queryClient.cancelQueries({ queryKey: ['servers', 'detail', serverId, 'icon'] })
		queryClient.setQueriesData({ queryKey: ['servers', 'detail', serverId, 'icon'] }, null)

		addNotification({
			type: 'success',
			title: formatMessage(messages.resetTitle),
			text: formatMessage(messages.resetText),
		})
	} catch {
		addNotification({
			type: 'error',
			title: formatMessage(messages.resetFailedTitle),
			text: formatMessage(messages.resetFailedText),
		})
	} finally {
		isSyncingIcon.value = false
	}
}

const triggerFileInput = () => {
	if (isIconActionDisabled.value) return

	const input = document.createElement('input')
	input.type = 'file'
	input.id = 'server-icon-field'
	input.accept = 'image/png,image/jpeg,image/gif,image/webp'
	const cleanup = () => {
		input.remove()
		window.removeEventListener('focus', handleWindowFocus)
	}
	const handleWindowFocus = () => {
		// If picker was cancelled there is no change event; clean up on focus return.
		setTimeout(() => {
			if (!input.value) cleanup()
		}, 0)
	}
	input.onchange = async (event) => {
		try {
			await uploadFile(event)
		} finally {
			cleanup()
		}
	}
	document.body.appendChild(input)
	window.addEventListener('focus', handleWindowFocus, { once: true })
	input.click()
}
</script>
