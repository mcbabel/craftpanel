<template>
	<div class="flex flex-col gap-4 max-md:gap-3 max-md:pb-16">
		<PageHeader
			:title="server.name"
			:divider="false"
			:bottom-padding="false"
			title-class="max-md:text-xl"
			row-class="max-md:gap-3"
		>
			<template #leading>
				<PanelServerIcon :server-id="server.id" class="max-md:size-12" />
			</template>

			<template #badges>
				<Badge :type="formatMessage(STATE_LABELS[powerState])" :color="STATE_COLORS[powerState]" />
				<CopyCode v-if="address" :text="address" />
			</template>

			<template #metadata>
				<ServerInfoLabels
					:server-data="vendorServer"
					:show-game-label="server.game_version !== null"
					:show-loader-label="server.loader !== null"
					:uptime-seconds="uptimeSeconds"
					class="flex w-full flex-row flex-wrap items-center gap-2 text-secondary"
				/>
			</template>

			<template #actions>
				<PageHeaderActions>
					<PanelServerActionButton :disabled="installError !== null" />
				</PageHeaderActions>
			</template>
		</PageHeader>

		<div class="isolate flex w-full select-none flex-col gap-4 overflow-auto max-md:hidden">
			<NavTabs :links="tabs" replace />
		</div>

		<ServerTabBar :links="tabs" :label="formatMessage(messages.sections)" />

		<ServerNotice
			v-if="notice"
			:level="notice.level"
			:message="notice.message"
			:dismissable="false"
		/>

		<Admonition
			v-if="connection === 'reconnecting'"
			type="warning"
			:header="formatMessage(messages.reconnectingTitle)"
			:body="formatMessage(messages.reconnectingBody)"
		/>
		<Admonition
			v-else-if="connection === 'lost'"
			type="critical"
			:header="formatMessage(messages.disconnectedTitle)"
			:body="formatMessage(messages.disconnectedBody)"
		>
			<template #actions>
				<Button type="colored" color="red" @click="socket.connect()">
					<UpdatedIcon />
					{{ formatMessage(messages.reconnect) }}
				</Button>
			</template>
		</Admonition>

		<ServerPanelAdmonitions @installation-retry="retryInstall" />

		<RouterView />
	</div>
</template>

<script setup lang="ts">
import {
	BoxesIcon,
	DatabaseBackupIcon,
	FolderOpenIcon,
	LayoutTemplateIcon,
	SettingsIcon,
	UpdatedIcon,
	UsersIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Badge,
	Button,
	commonMessages,
	CopyCode,
	defineMessages,
	injectNotificationManager,
	type MessageDescriptor,
	NavTabs,
	PageHeader,
	PageHeaderActions,
	PanelServerActionButton,
	ServerInfoLabels,
	ServerNotice,
	ServerPanelAdmonitions,
	useVIntl,
} from '@modrinth/ui'
import { computed, onScopeDispose, watch } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'

import { api, isApiRequestError, type PowerState, type Server, ServerSocket } from '@/api'
import { serverAddress } from '@/api/address'
import { publicAddress } from '@/api/playit'
import PanelServerIcon from '@/components/PanelServerIcon.vue'
import ServerTabBar from '@/components/ServerTabBar.vue'
import { provideArchonAdapters } from '@/composables/archon-adapters'
import { useServerTunnel } from '@/composables/playit'
import { provideServerPage } from '@/composables/server-page'
import { useSession } from '@/composables/session'
import { provideServerContext } from '@/providers'
import { useConsoleManager } from '@/providers/console-manager'

import { serverNotice } from './server-notice'

const props = defineProps<{ server: Server }>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const route = useRoute()
const router = useRouter()
const session = useSession()

const socket = new ServerSocket(props.server.id)
const page = provideServerContext({ server: props.server, socket })
provideServerPage({ ...page, serverId: props.server.id, socket })

provideArchonAdapters(props.server.id)
socket.connect()
onScopeDispose(() => socket.close())

const { context, installError, installOperation, server, socketStatus } = page
const { operations, refreshOperations } = page
const { powerState, uptimeSeconds } = context
const vendorServer = context.server

useConsoleManager({
	serverId: props.server.id,
	socket,
	powerState: () => powerState.value,
	permissions: () => server.value.current_user_permissions,
})

const STATE_LABELS: Record<PowerState, MessageDescriptor> = defineMessages({
	stopped: { id: 'panel.server.power.stopped', defaultMessage: 'Stopped' },
	starting: { id: 'panel.server.power.starting', defaultMessage: 'Starting' },
	running: { id: 'panel.server.power.running', defaultMessage: 'Running' },
	stopping: { id: 'panel.server.power.stopping', defaultMessage: 'Stopping' },
	crashed: { id: 'panel.server.power.crashed', defaultMessage: 'Crashed' },
})

const STATE_COLORS: Record<PowerState, string> = {
	stopped: 'gray',
	starting: 'orange',
	running: 'green',
	stopping: 'orange',
	crashed: 'red',
}

const messages = defineMessages({
	overview: {
		id: 'panel.server.tabs.overview',
		defaultMessage: 'Overview',
	},
	files: {
		id: 'panel.server.tabs.files',
		defaultMessage: 'Files',
	},
	backups: {
		id: 'panel.server.tabs.backups',
		defaultMessage: 'Backups',
	},
	access: {
		id: 'panel.server.tabs.access',
		defaultMessage: 'Access',
	},
	setupPending: {
		id: 'panel.server.notice.setup-pending',
		defaultMessage:
			'This server has no installation yet. Pick a loader and a version under Settings.',
	},
	broken: {
		id: 'panel.server.notice.broken',
		defaultMessage:
			'The last installation did not finish, so this server cannot start. Install it again ' +
			'under Settings.',
	},
	deleting: {
		id: 'panel.server.notice.deleting',
		defaultMessage: 'This server is being deleted.',
	},
	deleteFailed: {
		id: 'panel.server.notice.delete-failed',
		defaultMessage:
			'This server could not be deleted: {reason} Its files are still there and still count ' +
			'against your disk. Delete it again under Settings.',
	},
	reconnectingTitle: {
		id: 'panel.server.connection.reconnecting.title',
		defaultMessage: 'Reconnecting to the server',
	},
	reconnectingBody: {
		id: 'panel.server.connection.reconnecting.body',
		defaultMessage: 'The live connection dropped. Console and metrics stand still until it is back.',
	},
	disconnectedTitle: {
		id: 'panel.server.connection.lost.title',
		defaultMessage: 'No live connection',
	},
	disconnectedBody: {
		id: 'panel.server.connection.lost.body',
		defaultMessage: 'Console, metrics and progress are no longer updating.',
	},
	reconnect: {
		id: 'panel.server.connection.reconnect',
		defaultMessage: 'Reconnect',
	},
	accessRemoved: {
		id: 'panel.server.gone.access-removed',
		defaultMessage: 'Your access to this server was removed.',
	},
	deleted: {
		id: 'panel.server.gone.deleted',
		defaultMessage: 'This server has been deleted.',
	},
	sections: {
		id: 'panel.server.sections',
		defaultMessage: 'Server sections',
	},
	backupsShort: {
		id: 'panel.server.tab.backups-short',
		defaultMessage: 'Backups',
	},
	settingsShort: {
		id: 'panel.server.tab.settings-short',
		defaultMessage: 'Settings',
	},
	retryFailed: {
		id: 'panel.server.install.retry-failed',
		defaultMessage: 'The installation could not be started again.',
	},
})

const base = computed(() => `/servers/${encodeURIComponent(server.value.id)}`)
const setupPending = computed(() => server.value.flows.intro)

const browsePages = computed(() => [`${base.value}/browse`])
const browsingModpacks = computed(() => route.name === 'server-browse' && route.query.type === 'modpack')

const tabs = computed(() => [
	{
		label: formatMessage(messages.overview),
		href: base.value,
		icon: LayoutTemplateIcon,
	},
	{
		label: formatMessage(commonMessages.contentLabel),
		href: `${base.value}/content`,
		icon: BoxesIcon,
		subpages: browsingModpacks.value ? [] : browsePages.value,
	},
	{
		label: formatMessage(messages.files),
		href: `${base.value}/files`,
		icon: FolderOpenIcon,
	},
	{
		label: formatMessage(messages.backups),
		shortLabel: formatMessage(messages.backupsShort),
		href: `${base.value}/backups`,
		icon: DatabaseBackupIcon,
	},
	{
		label: formatMessage(messages.access),
		href: `${base.value}/access`,
		icon: UsersIcon,
	},
	{
		label: formatMessage(commonMessages.settingsLabel),
		shortLabel: formatMessage(messages.settingsShort),
		href: `${base.value}/settings`,
		icon: SettingsIcon,
		subpages: browsingModpacks.value ? browsePages.value : [],
	},
])

const notice = computed(() => {
	const seen = serverNotice(server.value.status, setupPending.value, operations.value)
	switch (seen?.kind) {
		case 'delete-failed':
			return {
				level: 'critical',
				message: formatMessage(messages.deleteFailed, { reason: seen.reason }),
			}
		case 'deleting':
			return { level: 'critical', message: formatMessage(messages.deleting) }
		case 'broken':
			return { level: 'warn', message: formatMessage(messages.broken) }
		case 'setup-pending':
			return { level: 'info', message: formatMessage(messages.setupPending) }
		default:
			return null
	}
})

const { tunnel } = useServerTunnel(props.server.id)
const address = computed(() => publicAddress(tunnel.value) ?? serverAddress(server.value.net))

const GONE_CODES = new Set([4401, 4403, 4404])

const connection = computed(() => {
	const status = socketStatus.value
	if (status.phase === 'open') return 'connected'
	if (status.closeCode !== null && GONE_CODES.has(status.closeCode)) return 'gone'
	if (status.givenUp || (status.phase === 'closed' && status.closeCode !== null)) return 'lost'
	return status.attempts > 0 ? 'reconnecting' : 'connecting'
})

watch(socketStatus, (status) => {
	if (status.closeCode === null || !GONE_CODES.has(status.closeCode)) return

	if (status.closeCode === 4401) {
		session.forget()
		void router.replace({ name: 'login', query: { redirect: route.fullPath } })
		return
	}

	addNotification({
		title: formatMessage(status.closeCode === 4404 ? messages.deleted : messages.accessRemoved),
		text: server.value.name,
		type: 'warning',
	})
	void router.replace({ name: 'servers' })
})

async function retryInstall(): Promise<void> {
	const operation = installOperation.value
	if (operation === null) return

	try {
		await api.operations.retry(server.value.id, operation.id)
		await refreshOperations()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.retryFailed),
			text: isApiRequestError(error) ? error.message : String(error),
			type: 'error',
		})
	}
}
</script>
