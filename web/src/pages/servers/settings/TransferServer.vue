<template>
	<div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
		<div class="flex flex-col gap-1">
			<span class="font-semibold text-contrast">{{ formatMessage(messages.title) }}</span>
			<span v-if="waiting">
				{{ formatMessage(messages.waiting, { name: waiting.to.username }) }}
			</span>
			<span v-else>
				{{ formatMessage(isAdmin ? messages.descriptionAdmin : messages.description) }}
			</span>
		</div>
		<Button v-if="waiting" :disabled="working" @click="() => void withdraw()">
			<XIcon class="size-5" aria-hidden="true" />
			{{ formatMessage(messages.withdraw) }}
		</Button>
		<Button v-else v-tooltip="disabledReason" :disabled="!!disabledReason" @click="open">
			<TransferIcon class="size-5" aria-hidden="true" />
			{{ formatMessage(messages.title) }}
		</Button>
	</div>

	<Teleport to="body">
		<NewModal ref="modal" :header="formatMessage(messages.title)" width="30rem">
			<form class="flex flex-col gap-4" @submit.prevent="() => void submit()">
				<p class="m-0">
					{{
						formatMessage(isAdmin ? messages.explainAdmin : messages.explain, { name: serverName })
					}}
				</p>
				<label class="flex flex-col gap-2">
					<span class="font-semibold text-contrast">{{ formatMessage(messages.receiver) }}</span>
					<Input
						v-model="receiver"
						:placeholder="formatMessage(messages.receiverPlaceholder)"
						autocomplete="off"
						autofocus
					/>
				</label>
				<Admonition
					v-if="refusal"
					:type="refusal.budget ? 'warning' : 'critical'"
					:header="refusal.budget ? formatMessage(messages.noRoom) : undefined"
					:body="refusal.text"
				/>
				<div class="mb-1 flex justify-end gap-2.5">
					<Button :disabled="working" @click="modal?.hide()">
						{{ formatMessage(commonMessages.cancelButton) }}
					</Button>
					<Button
						type="colored"
						color="brand"
						native-type="submit"
						:disabled="working || receiver.trim() === ''"
					>
						<SpinnerIcon v-if="working" class="animate-spin" aria-hidden="true" />
						<TransferIcon v-else aria-hidden="true" />
						{{ formatMessage(isAdmin ? messages.moveNow : messages.send) }}
					</Button>
				</div>
			</form>
		</NewModal>
	</Teleport>
</template>

<script setup lang="ts">
import { SpinnerIcon, TransferIcon, XIcon } from '@modrinth/assets'
import {
	Admonition,
	Button,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	Input,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import { api, hasErrorCode, isApiRequestError, type ServerTransfer, type Ulid } from '@/api'
import { useSession } from '@/composables/session'

const props = defineProps<{
	serverId: Ulid
	serverName: string
	disabledReason?: string
}>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const { isAdmin } = useSession()
const router = useRouter()

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const receiver = ref('')
const working = ref(false)
const refusal = ref<{ text: string; budget: boolean } | null>(null)
const outgoing = ref<ServerTransfer[]>([])

const waiting = computed(() => outgoing.value.find((transfer) => transfer.server.id === props.serverId))

const messages = defineMessages({
	title: { id: 'panel.transfer.title', defaultMessage: 'Transfer to another account' },
	description: {
		id: 'panel.transfer.description',
		defaultMessage: 'Offer this server to someone else. It only moves once they accept.',
	},
	descriptionAdmin: {
		id: 'panel.transfer.description-admin',
		defaultMessage: 'Give this server to another account. As an administrator you move it right away.',
	},
	explain: {
		id: 'panel.transfer.explain',
		defaultMessage:
			'{name} is offered to the account you name. It stays yours until they accept, and their budget has to have room for it.',
	},
	explainAdmin: {
		id: 'panel.transfer.explain-admin',
		defaultMessage:
			'{name} moves to the account you name right away, files and all. Their budget still has to have room for it.',
	},
	receiver: { id: 'panel.transfer.receiver', defaultMessage: 'Username' },
	receiverPlaceholder: { id: 'panel.transfer.receiver-placeholder', defaultMessage: 'Who gets it?' },
	send: { id: 'panel.transfer.send', defaultMessage: 'Send offer' },
	moveNow: { id: 'panel.transfer.move-now', defaultMessage: 'Transfer now' },
	noRoom: { id: 'panel.transfer.no-room', defaultMessage: 'Not enough room in their budget' },
	waiting: {
		id: 'panel.transfer.waiting',
		defaultMessage: 'Offered to {name}. It moves once they accept.',
	},
	withdraw: { id: 'panel.transfer.withdraw', defaultMessage: 'Withdraw offer' },
	sent: { id: 'panel.transfer.sent', defaultMessage: 'Offer sent to {name}' },
	moved: { id: 'panel.transfer.moved', defaultMessage: '{server} now belongs to {name}' },
	withdrawn: { id: 'panel.transfer.withdrawn', defaultMessage: 'Offer withdrawn' },
	failed: { id: 'panel.transfer.failed', defaultMessage: 'The transfer could not be changed' },
})

async function refresh(): Promise<void> {
	try {
		outgoing.value = (await api.serverTransfers.list()).outgoing
	} catch {
		outgoing.value = []
	}
}

function open(): void {
	receiver.value = ''
	refusal.value = null
	modal.value?.show()
}

async function submit(): Promise<void> {
	const to = receiver.value.trim()
	if (!to || working.value) return
	working.value = true
	refusal.value = null
	try {
		const answer = await api.serverTransfers.start(props.serverId, to)
		modal.value?.hide()
		if (answer.outcome === 'transferred') {
			addNotification({
				type: 'success',
				title: formatMessage(messages.moved, { server: props.serverName, name: answer.owner.username }),
			})
			await router.push({ name: 'servers' })
			return
		}
		addNotification({ type: 'success', title: formatMessage(messages.sent, { name: answer.transfer.to.username }) })
		await refresh()
	} catch (cause) {
		refusal.value = {
			text: isApiRequestError(cause) || cause instanceof Error ? cause.message : String(cause),
			budget: hasErrorCode(cause, 'receiver_memory_budget', 'receiver_disk_budget'),
		}
	} finally {
		working.value = false
	}
}

async function withdraw(): Promise<void> {
	const transfer = waiting.value
	if (!transfer) return
	working.value = true
	try {
		await api.serverTransfers.withdraw(transfer.id)
		addNotification({ type: 'success', title: formatMessage(messages.withdrawn) })
	} catch (cause) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.failed),
			text: cause instanceof Error ? cause.message : undefined,
		})
	} finally {
		working.value = false
		await refresh()
	}
}

onMounted(() => void refresh())
</script>
