<template>
	<section v-if="incoming.length > 0" class="flex flex-col gap-3">
		<div class="flex flex-col gap-1">
			<h2 class="m-0 text-lg font-semibold text-secondary">
				{{ formatMessage(messages.title) }}
			</h2>
			<span class="text-secondary">{{ formatMessage(messages.help) }}</span>
		</div>

		<div
			v-for="transfer in incoming"
			:key="transfer.id"
			class="flex flex-col gap-4 rounded-2xl border-[1px] border-solid border-surface-4 bg-bg-raised p-4 sm:flex-row sm:items-center"
		>
			<div class="flex min-w-0 flex-row items-center gap-4">
				<ServerIcon :image="undefined" />

				<div class="flex min-w-0 flex-col gap-1.5">
					<h3 class="m-0 min-w-0 truncate text-xl font-bold text-contrast">
						{{ transfer.server.name }}
					</h3>
					<div class="flex min-w-0 flex-row flex-wrap items-center gap-1.5 text-secondary">
						<span>{{ formatMessage(messages.offeredBy) }}</span>
						<span
							class="flex min-w-0 items-center gap-1 rounded-full border border-solid border-surface-5 bg-surface-4 px-2 py-1 pr-2.5 text-sm font-medium text-primary"
						>
							<Avatar :tint-by="transfer.from.username" size="1.25rem" circle no-shadow />
							<span class="max-w-32 truncate">{{ transfer.from.username }}</span>
						</span>
						<span v-tooltip="formatDateTime(transfer.created_at)" class="text-sm">
							{{ formatRelativeTime(transfer.created_at) }}
						</span>
					</div>
					<span class="text-sm text-secondary">
						{{ formatMessage(messages.claims, { memory: formatBytes(transfer.server.memory_mib * 1024 * 1024, 0) }) }}
					</span>
				</div>
			</div>

			<div class="flex shrink-0 flex-row flex-wrap gap-2 sm:ml-auto">
				<Button
					type="colored"
					color="brand"
					:disabled="busy[transfer.id] !== undefined"
					@click="() => void accept(transfer)"
				>
					<SpinnerIcon v-if="busy[transfer.id] === 'accept'" class="animate-spin" aria-hidden="true" />
					<CheckIcon v-else aria-hidden="true" />
					{{ formatMessage(messages.accept) }}
				</Button>
				<Button
					type="outlined"
					:disabled="busy[transfer.id] !== undefined"
					@click="() => void decline(transfer)"
				>
					<SpinnerIcon v-if="busy[transfer.id] === 'decline'" class="animate-spin" aria-hidden="true" />
					<XIcon v-else aria-hidden="true" />
					{{ formatMessage(messages.decline) }}
				</Button>
			</div>
		</div>
	</section>
</template>

<script setup lang="ts">
import { CheckIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	Avatar,
	Button,
	defineMessages,
	injectNotificationManager,
	ServerIcon,
	useFormatBytes,
	useFormatDateTime,
	useRelativeTime,
	useVIntl,
} from '@modrinth/ui'
import { onMounted, ref } from 'vue'

import { api, isApiRequestError, type ServerTransfer, type Ulid } from '@/api'

const emit = defineEmits<{ accepted: [serverId: Ulid] }>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const formatBytes = useFormatBytes()
const formatRelativeTime = useRelativeTime()
const formatDateTime = useFormatDateTime({ dateStyle: 'medium', timeStyle: 'short' })

const incoming = ref<ServerTransfer[]>([])
const busy = ref<Record<Ulid, 'accept' | 'decline'>>({})

const messages = defineMessages({
	title: { id: 'panel.transfers.title', defaultMessage: 'Servers offered to you' },
	help: {
		id: 'panel.transfers.help',
		defaultMessage: 'Accept one to become its owner. Its memory then counts against your budget.',
	},
	offeredBy: { id: 'panel.transfers.offered-by', defaultMessage: 'Offered by' },
	claims: { id: 'panel.transfers.claims', defaultMessage: 'Claims {memory} of memory' },
	accept: { id: 'panel.transfers.accept', defaultMessage: 'Accept' },
	decline: { id: 'panel.transfers.decline', defaultMessage: 'Decline' },
	accepted: { id: 'panel.transfers.accepted', defaultMessage: '{name} is yours now' },
	declined: { id: 'panel.transfers.declined', defaultMessage: 'Transfer declined' },
	failed: { id: 'panel.transfers.failed', defaultMessage: 'The transfer could not be answered' },
})

async function refresh(): Promise<void> {
	try {
		incoming.value = (await api.serverTransfers.list()).incoming
	} catch {
		incoming.value = []
	}
}

function report(cause: unknown): void {
	addNotification({
		type: 'error',
		title: formatMessage(messages.failed),
		text: isApiRequestError(cause) || cause instanceof Error ? cause.message : String(cause),
	})
}

async function accept(transfer: ServerTransfer): Promise<void> {
	busy.value = { ...busy.value, [transfer.id]: 'accept' }
	try {
		await api.serverTransfers.accept(transfer.id)
		addNotification({ type: 'success', title: formatMessage(messages.accepted, { name: transfer.server.name }) })
		emit('accepted', transfer.server.id)
	} catch (cause) {
		report(cause)
	} finally {
		delete busy.value[transfer.id]
		await refresh()
	}
}

async function decline(transfer: ServerTransfer): Promise<void> {
	busy.value = { ...busy.value, [transfer.id]: 'decline' }
	try {
		await api.serverTransfers.withdraw(transfer.id)
		addNotification({ type: 'success', title: formatMessage(messages.declined) })
	} catch (cause) {
		report(cause)
	} finally {
		delete busy.value[transfer.id]
		await refresh()
	}
}

onMounted(() => void refresh())

defineExpose({ refresh })
</script>
