<template>
	<InstallationSettingsLayout @reset-server="openReset">
		<template #extra>
			<div class="flex flex-col gap-2.5">
				<span class="text-lg font-semibold text-contrast">
					{{ formatMessage(messages.resetTitle) }}
				</span>
				<div>
					<Button
						v-tooltip="resetTooltip"
						type="colored"
						color="red"
						:disabled="resetDisabled"
						@click="openReset"
					>
						<RotateCounterClockwiseIcon class="size-5" />
						{{ formatMessage(commonMessages.resetServerButton) }}
					</Button>
				</div>
				<span class="text-primary">{{ formatMessage(messages.resetDescription) }}</span>
			</div>
		</template>
	</InstallationSettingsLayout>

	<ServerSetupFlow
		ref="setup"
		type="reset-server"
		:initial-loader="initialLoader"
		:initial-game-version="initialGameVersion"
		:finish-disabled="!canResetServer"
		:finish-disabled-tooltip="canResetServer ? undefined : permissionDeniedMessage"
		:submit="reset"
		@browse-modpacks="browseModpacks"
	/>
</template>

<script setup lang="ts">
import { RotateCounterClockwiseIcon } from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	injectModrinthClient,
	injectNotificationManager,
	InstallationSettingsLayout,
	useServerPermissions,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, useTemplateRef } from 'vue'
import { useRouter } from 'vue-router'

import { api, type Operation, type PropertiesFields } from '@/api'
import ServerSetupFlow from '@/components/ServerSetupFlow.vue'
import { useServerPage } from '@/composables/server-page'
import { awaitOperation } from '@/providers/content-manager'
import { type SetupChoice, takeModpackPick } from '@/providers/creation-catalog'
import { useInstallationSettings } from '@/providers/installation-settings'

const { formatMessage } = useVIntl()
const router = useRouter()
const client = injectModrinthClient()
const { addNotification } = injectNotificationManager()
const { serverId, server, socket, context } = useServerPage()
const { canResetServer, permissionDeniedMessage } = useServerPermissions()

const messages = defineMessages({
	resetTitle: {
		id: 'craftpanel.installation.reset-title',
		defaultMessage: 'Reset server',
	},
	resetDescription: {
		id: 'craftpanel.installation.reset-description',
		defaultMessage:
			'Removes everything on the server, including the world, mods and configuration, and sets it up again with a loader or a modpack you choose. Backups stay and can be restored.',
	},
	running: {
		id: 'craftpanel.installation.reset-running',
		defaultMessage: 'Stop the server first.',
	},
	resetStarted: {
		id: 'craftpanel.installation.reset-started',
		defaultMessage: 'The server is being reset',
	},
	propertiesFailed: {
		id: 'craftpanel.installation.reset-properties-failed',
		defaultMessage: 'The world settings could not be written',
	},
})

const setup = useTemplateRef<InstanceType<typeof ServerSetupFlow>>('setup')

const busyReasons = context.busyReasons
const running = computed(() => {
	const state = context.powerState.value
	return state !== 'stopped' && state !== 'crashed'
})
const resetDisabled = computed(
	() => !canResetServer.value || running.value || busyReasons.value.length > 0,
)
const resetTooltip = computed(() => {
	if (!canResetServer.value) return permissionDeniedMessage.value
	if (running.value) return formatMessage(messages.running)
	if (busyReasons.value.length > 0) return formatMessage(busyReasons.value[0].reason)
	return undefined
})

const initialLoader = computed(() => {
	const loader = server.value?.loader
	return loader && loader !== 'vanilla' ? loader : undefined
})
const initialGameVersion = computed(() => server.value?.game_version ?? undefined)

function openReset(): void {
	if (resetDisabled.value) return
	void setup.value?.show()
}

function browseModpacks(): void {
	void router.push({ name: 'server-browse', params: { id: serverId }, query: { type: 'modpack' } })
}

async function writeProperties(started: Operation, properties: PropertiesFields): Promise<void> {
	const host = { serverId, socket, client: api }
	const finished = await awaitOperation(host, started)
	if (finished.state !== 'done') return
	const known = Object.fromEntries(
		Object.entries(properties.known).filter(([, value]) => value !== undefined),
	) as Record<string, string | null>
	try {
		await api.settings.patchProperties(serverId, { known })
	} catch (cause) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.propertiesFailed),
			text: cause instanceof Error ? cause.message : undefined,
		})
	}
}

async function reset(choice: SetupChoice): Promise<boolean> {
	if (resetDisabled.value) return false
	switch (choice.kind) {
		case 'loader': {
			if (installation.context.isLinked.value) await api.content.unlinkModpack(serverId)
			const accepted = await api.settings.reset(serverId, {
				loader: choice.loader,
				game_version: choice.gameVersion,
				loader_version: choice.loaderVersion,
				keep_backups: true,
			})
			void writeProperties(accepted.operation, choice.properties)
			break
		}
		case 'modpack':
			await api.content.installModpack(serverId, {
				source: { kind: 'modrinth', project_id: choice.projectId, version_id: choice.versionId },
				keep_extra_content: false,
				reset: true,
				properties: choice.properties,
			})
			break
		case 'upload':
			await api.content.uploadModpack(serverId, choice.file, false, {
				reset: choice.properties,
			}).promise
			break
	}
	addNotification({ type: 'success', title: formatMessage(messages.resetStarted) })
	return true
}

const installation = useInstallationSettings({
	serverId,
	server,
	socket,
	busyReasons,
	notify: addNotification,
	modrinth: client.labrinth.versions_v2,
})

onMounted(() => {
	const pick = takeModpackPick()
	if (!pick) return
	if (!resetDisabled.value) void setup.value?.showWithModpack(pick)
})
</script>
