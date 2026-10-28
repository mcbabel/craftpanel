<template>
	<div class="flex flex-col gap-2.5">
		<span class="text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</span>
		<span>{{ formatMessage(messages.intro) }}</span>

		<LoadingIndicator v-if="loading && entries.length === 0" />
		<div
			v-for="entry in entries"
			:key="entry.id"
			class="flex flex-wrap items-center gap-3 rounded-xl bg-surface-2 px-4 py-3"
		>
			<TagItem class="!font-semibold">{{ formatMessage(kindLabels[entry.kind]) }}</TagItem>
			<span class="font-semibold text-contrast">{{ entry.label ?? portName(entry.local_port) }}</span>
			<span class="text-sm text-secondary">
				{{ formatMessage(messages.localPort, { port: entry.local_port ?? 0 }) }}
			</span>
			<CopyCode v-if="addressOf(entry)" :text="addressOf(entry)!" />
			<span v-else class="text-sm" :class="entry.state === 'failed' ? 'text-red' : 'text-secondary'">
				{{ stateText(entry) }}
			</span>
			<IconButton
				v-if="canManage"
				class="ml-auto"
				type="quiet"
				:label="formatMessage(messages.unpublish)"
				:disabled="busy"
				@click="() => void remove(entry)"
			>
				<TrashIcon />
			</IconButton>
		</div>

		<form v-if="canManage" class="flex flex-col gap-3 rounded-xl border border-dashed border-surface-5 p-4" @submit.prevent="() => void publish()">
			<span class="font-semibold text-contrast">{{ formatMessage(messages.addTitle) }}</span>
			<Chips
				v-model="kind"
				:items="KINDS"
				:format-label="(choice: Choice) => formatMessage(kindLabels[choice])"
				:capitalize="false"
				:aria-label="formatMessage(messages.kind)"
			/>
			<span class="text-sm text-secondary">{{ formatMessage(kindHints[kind]) }}</span>
			<div class="flex flex-wrap items-center gap-2">
				<DropdownSelect
					v-if="portChoices.length > 0"
					:model-value="port"
					:options="portChoices"
					:display-name="(value: number) => `${portName(value)} · ${value}`"
					name="playit-port"
					class="!w-60"
					@update:model-value="(value: unknown) => (port = Number(value))"
				/>
				<span v-else class="text-sm text-secondary">{{ formatMessage(messages.noPort) }}</span>
				<Input
					v-model="label"
					:maxlength="32"
					:placeholder="formatMessage(messages.labelPlaceholder)"
					wrapper-class="w-56"
				/>
				<Button
					type="colored"
					color="brand"
					native-type="submit"
					:disabled="busy || port === null || portChoices.length === 0"
				>
					<GlobeIcon aria-hidden="true" />
					{{ formatMessage(messages.publish) }}
				</Button>
			</div>
		</form>

		<span class="text-sm text-secondary">{{ formatMessage(messages.voiceChat) }}</span>
	</div>
</template>

<script setup lang="ts">
import { GlobeIcon, TrashIcon } from '@modrinth/assets'
import {
	Button,
	Chips,
	CopyCode,
	defineMessages,
	DropdownSelect,
	IconButton,
	injectNotificationManager,
	Input,
	LoadingIndicator,
	type MessageDescriptor,
	TagItem,
	useVIntl,
} from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import type { Ulid } from '@/api'
import {
	playit,
	type PlayitTunnelKind,
	type PublishedPort,
	type PublishPortRequest,
	shareAddress,
} from '@/api/playit'

type Choice = PublishPortRequest['kind']

interface HeldPort {
	port: number
	name: string
	primary: boolean
}

const props = defineProps<{
	serverId: Ulid
	ports: HeldPort[]
	canManage: boolean
}>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()

const KINDS: Choice[] = ['udp', 'tcp', 'minecraft-bedrock']

const entries = ref<PublishedPort[]>([])
const loading = ref(false)
const busy = ref(false)
const kind = ref<Choice>('udp')
const port = ref<number | null>(null)
const label = ref('')
let timer: ReturnType<typeof setTimeout> | null = null

const messages = defineMessages({
	title: { id: 'craftpanel.network.playit.title', defaultMessage: 'More ports through playit.gg' },
	intro: {
		id: 'craftpanel.network.playit.intro',
		defaultMessage:
			'Next to the game address, other ports of this server can get their own public address: a voice chat, a web map, a port for Bedrock players.',
	},
	localPort: { id: 'craftpanel.network.playit.local-port', defaultMessage: 'Port {port} here' },
	pending: { id: 'craftpanel.network.playit.pending', defaultMessage: 'Waiting for playit.gg to hand out an address...' },
	offline: { id: 'craftpanel.network.playit.offline', defaultMessage: 'Offline at playit.gg' },
	failed: { id: 'craftpanel.network.playit.failed', defaultMessage: 'Failed' },
	missing: { id: 'craftpanel.network.playit.missing', defaultMessage: 'Removed on playit.gg' },
	unpublish: { id: 'craftpanel.network.playit.unpublish', defaultMessage: 'Take the address away' },
	addTitle: { id: 'craftpanel.network.playit.add-title', defaultMessage: 'Publish a port' },
	kind: { id: 'craftpanel.network.playit.kind', defaultMessage: 'Kind' },
	noPort: {
		id: 'craftpanel.network.playit.no-port',
		defaultMessage: 'Add a port to this server first, under ports above.',
	},
	labelPlaceholder: { id: 'craftpanel.network.playit.label', defaultMessage: 'Label, e.g. Voice chat' },
	publish: { id: 'craftpanel.network.playit.publish', defaultMessage: 'Publish' },
	published: { id: 'craftpanel.network.playit.published', defaultMessage: 'Asked playit.gg for an address' },
	removed: { id: 'craftpanel.network.playit.removed', defaultMessage: 'Address taken away' },
	actionFailed: { id: 'craftpanel.network.playit.action-failed', defaultMessage: 'playit.gg did not take it' },
	voiceChat: {
		id: 'craftpanel.network.playit.voice-chat',
		defaultMessage:
			'Simple Voice Chat: add a port above, publish it here as UDP, then in config/voicechat/voicechat-server.properties set port to that port and voice_host to the address shown here, and restart the server.',
	},
})

const kindLabels: Record<PlayitTunnelKind, MessageDescriptor> = defineMessages({
	'minecraft-java': { id: 'craftpanel.network.playit.kind.java', defaultMessage: 'Minecraft Java' },
	'minecraft-bedrock': { id: 'craftpanel.network.playit.kind.bedrock', defaultMessage: 'Bedrock (UDP)' },
	tcp: { id: 'craftpanel.network.playit.kind.tcp', defaultMessage: 'TCP' },
	udp: { id: 'craftpanel.network.playit.kind.udp', defaultMessage: 'UDP' },
})

const kindHints: Record<Choice, MessageDescriptor> = defineMessages({
	udp: {
		id: 'craftpanel.network.playit.hint.udp',
		defaultMessage: 'For voice chats like Simple Voice Chat and other UDP services.',
	},
	tcp: {
		id: 'craftpanel.network.playit.hint.tcp',
		defaultMessage: 'For web maps like BlueMap or Dynmap and other TCP services. The primary port is published on the overview page.',
	},
	'minecraft-bedrock': {
		id: 'craftpanel.network.playit.hint.bedrock',
		defaultMessage: 'For Bedrock players through Geyser. Pick the port Geyser listens on; it may be the primary port.',
	},
})

const portChoices = computed(() =>
	props.ports
		.filter((held) => kind.value !== 'tcp' || !held.primary)
		.sort((left, right) => Number(left.primary) - Number(right.primary))
		.map((held) => held.port),
)

watch(
	portChoices,
	(choices) => {
		if (port.value === null || !choices.includes(port.value)) port.value = choices[0] ?? null
	},
	{ immediate: true },
)

function portName(value: number | null): string {
	return props.ports.find((held) => held.port === value)?.name ?? String(value ?? '')
}

function addressOf(entry: PublishedPort): string | null {
	if (entry.state !== 'online') return null
	return shareAddress(entry)?.address ?? null
}

function stateText(entry: PublishedPort): string {
	switch (entry.state) {
		case 'offline':
			return entry.detail ?? formatMessage(messages.offline)
		case 'failed':
			return entry.detail ?? formatMessage(messages.failed)
		case 'missing':
			return formatMessage(messages.missing)
		default:
			return formatMessage(messages.pending)
	}
}

function reason(cause: unknown): string | undefined {
	return cause instanceof Error ? cause.message : undefined
}

function schedule(): void {
	if (timer !== null) clearTimeout(timer)
	const waiting = entries.value.some((entry) => entry.state === 'pending')
	timer = setTimeout(() => void load(), waiting ? 2_000 : 30_000)
}

async function load(): Promise<void> {
	loading.value = true
	try {
		entries.value = await playit.published(props.serverId)
	} catch {
		entries.value = []
	} finally {
		loading.value = false
		schedule()
	}
}

async function publish(): Promise<void> {
	if (port.value === null) return
	busy.value = true
	try {
		const trimmed = label.value.trim()
		entries.value = await playit.publish(props.serverId, {
			kind: kind.value,
			port: port.value,
			...(trimmed ? { label: trimmed } : {}),
		})
		label.value = ''
		addNotification({ type: 'success', title: formatMessage(messages.published) })
		schedule()
	} catch (cause) {
		addNotification({ type: 'error', title: formatMessage(messages.actionFailed), text: reason(cause) })
	} finally {
		busy.value = false
	}
}

async function remove(entry: PublishedPort): Promise<void> {
	busy.value = true
	try {
		await playit.unpublish(props.serverId, entry.id)
		addNotification({ type: 'success', title: formatMessage(messages.removed) })
		await load()
	} catch (cause) {
		addNotification({ type: 'error', title: formatMessage(messages.actionFailed), text: reason(cause) })
	} finally {
		busy.value = false
	}
}

onMounted(() => void load())
onBeforeUnmount(() => {
	if (timer !== null) clearTimeout(timer)
})
</script>
