<template>
	<section v-if="invitations.length > 0" class="flex flex-col gap-3">
		<div class="flex flex-col gap-1">
			<h2 class="m-0 text-lg font-semibold text-secondary">
				{{ formatMessage(messages.title) }}
			</h2>
			<span class="text-secondary">{{ formatMessage(messages.help) }}</span>
		</div>

		<div
			v-for="invitation in invitations"
			:key="invitation.id"
			class="flex flex-col gap-4 rounded-2xl border-[1px] border-solid border-surface-4 bg-bg-raised p-4 sm:flex-row sm:items-center"
		>
			<div class="flex min-w-0 flex-row items-center gap-4">
				<ServerIcon :image="undefined" />

				<div class="flex min-w-0 flex-col gap-1.5">
					<div class="flex min-w-0 flex-row flex-wrap items-center gap-2.5">
						<h3 class="m-0 min-w-0 truncate text-xl font-bold text-contrast">
							{{ invitation.server.name }}
						</h3>
						<Badge
							:type="formatMessage(ROLE_LABELS[invitation.role])"
							:color="ROLE_COLORS[invitation.role]"
						/>
					</div>

					<div class="flex min-w-0 flex-row flex-wrap items-center gap-1.5 text-secondary">
						<span>{{ formatMessage(messages.invitedBy) }}</span>
						<span
							class="flex min-w-0 items-center gap-1 rounded-full border border-solid border-surface-5 bg-surface-4 px-2 py-1 pr-2.5 text-sm font-medium text-primary"
						>
							<Avatar
								:src="invitation.invited_by.avatar_url"
								:tint-by="invitation.invited_by.username"
								size="1.25rem"
								circle
								no-shadow
							/>
							<span class="max-w-32 truncate">{{ invitation.invited_by.username }}</span>
						</span>
						<span v-tooltip="formatDateTime(invitation.invited_at)" class="text-sm">
							{{ formatRelativeTime(invitation.invited_at) }}
						</span>
					</div>

					<span v-if="ROLE_DESCRIPTIONS[invitation.role]" class="text-sm text-secondary">
						{{ formatMessage(ROLE_DESCRIPTIONS[invitation.role]!) }}
					</span>
				</div>
			</div>

			<div class="flex shrink-0 flex-row flex-wrap gap-2 sm:ml-auto">
				<Button
					type="colored"
					color="brand"
					:disabled="busy[invitation.id] !== undefined"
					@click="() => void accept(invitation)"
				>
					<SpinnerIcon
						v-if="busy[invitation.id] === 'accept'"
						class="animate-spin"
						aria-hidden="true"
					/>
					<CheckIcon v-else aria-hidden="true" />
					{{ formatMessage(messages.accept) }}
				</Button>
				<Button
					type="outlined"
					:disabled="busy[invitation.id] !== undefined"
					@click="() => void decline(invitation)"
				>
					<SpinnerIcon
						v-if="busy[invitation.id] === 'decline'"
						class="animate-spin"
						aria-hidden="true"
					/>
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
	Badge,
	Button,
	defineMessages,
	injectNotificationManager,
	ServerIcon,
	useFormatDateTime,
	useRelativeTime,
	useVIntl,
} from '@modrinth/ui'

import type { Invitation, Ulid } from '@/api'
import { useInvitations } from '@/composables/invitations'
import { ROLE_COLORS, ROLE_DESCRIPTIONS, ROLE_LABELS } from '@/pages/servers/roles'

const emit = defineEmits<{ accepted: [serverId: Ulid] }>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const formatRelativeTime = useRelativeTime()
const formatDateTime = useFormatDateTime({ dateStyle: 'medium', timeStyle: 'short' })
const inbox = useInvitations()
const { invitations, busy } = inbox

const messages = defineMessages({
	title: { id: 'panel.invitations.title', defaultMessage: 'Invitations' },
	help: {
		id: 'panel.invitations.help',
		defaultMessage: 'Accept an invitation to see and manage the server.',
	},
	invitedBy: { id: 'panel.invitations.invited-by', defaultMessage: 'Invited by' },
	accept: { id: 'panel.invitations.accept', defaultMessage: 'Accept' },
	decline: { id: 'panel.invitations.decline', defaultMessage: 'Decline' },
	accepted: {
		id: 'panel.invitations.accepted',
		defaultMessage: '{name} is now in your server list',
	},
	declined: { id: 'panel.invitations.declined', defaultMessage: 'Invitation declined' },
	failed: {
		id: 'panel.invitations.failed',
		defaultMessage: 'The invitation could not be answered',
	},
})

function report(cause: unknown): void {
	addNotification({
		type: 'error',
		title: formatMessage(messages.failed),
		text: cause instanceof Error ? cause.message : String(cause),
	})
}

async function accept(invitation: Invitation): Promise<void> {
	try {
		await inbox.accept(invitation.id)
		emit('accepted', invitation.server.id)
		addNotification({
			type: 'success',
			title: formatMessage(messages.accepted, { name: invitation.server.name }),
		})
	} catch (cause) {
		report(cause)
	}
}

async function decline(invitation: Invitation): Promise<void> {
	try {
		await inbox.decline(invitation.id)
		addNotification({ type: 'success', title: formatMessage(messages.declined) })
	} catch (cause) {
		report(cause)
	}
}
</script>
