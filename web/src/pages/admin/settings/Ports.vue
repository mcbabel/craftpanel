<template>
	<Card class="!mb-0 flex flex-col gap-5">
		<SettingsLabel :title="formatMessage(messages.title)" :description="formatMessage(messages.description)" />

		<div class="flex flex-col gap-2">
			<span class="text-sm font-semibold text-contrast">{{ formatMessage(messages.userPorts) }}</span>
			<Chips
				v-model="settings.user_ports"
				:items="CHOICES"
				:format-label="(choice: UserPorts) => formatMessage(labels[choice])"
				:aria-label="formatMessage(messages.userPorts)"
				:capitalize="false"
			/>
			<span class="text-sm text-secondary">{{ formatMessage(hints[settings.user_ports]) }}</span>
		</div>

		<div class="flex items-center justify-between gap-4">
			<label class="flex flex-col gap-0.5" for="network-needs-playit">
				<span class="font-semibold text-contrast">{{ formatMessage(messages.needsPlayit) }}</span>
				<span class="text-sm text-secondary">{{ formatMessage(messages.needsPlayitHint) }}</span>
			</label>
			<Toggle id="network-needs-playit" v-model="settings.network_needs_playit" />
		</div>
	</Card>
</template>

<script setup lang="ts">
import {
	Card,
	Chips,
	defineMessages,
	type MessageDescriptor,
	SettingsLabel,
	Toggle,
	useVIntl,
} from '@modrinth/ui'

import type { PanelSettings, UserPorts } from '@/api'

const { formatMessage } = useVIntl()

const settings = defineModel<PanelSettings>({ required: true })

const CHOICES: UserPorts[] = ['locked', 'pool', 'choose']

const messages = defineMessages({
	title: { id: 'admin.settings.ports.title', defaultMessage: 'Ports and the network tab' },
	description: {
		id: 'admin.settings.ports.description',
		defaultMessage: 'What users may do on the network tab of their servers. Administrators may always do everything.',
	},
	userPorts: { id: 'admin.settings.ports.user-ports', defaultMessage: 'Users and ports' },
	needsPlayit: {
		id: 'admin.settings.ports.needs-playit',
		defaultMessage: 'Network tab only with playit.gg',
	},
	needsPlayitHint: {
		id: 'admin.settings.ports.needs-playit-hint',
		defaultMessage:
			'Hide the network tab, from administrators too, on every server whose owner has no playit.gg account connected, for example when the ports are forwarded by hand anyway.',
	},
})

const labels: Record<UserPorts, MessageDescriptor> = defineMessages({
	locked: { id: 'admin.settings.ports.locked', defaultMessage: 'Locked' },
	pool: { id: 'admin.settings.ports.pool', defaultMessage: 'Picked by the panel' },
	choose: { id: 'admin.settings.ports.choose', defaultMessage: 'Pick a number' },
})

const hints: Record<UserPorts, MessageDescriptor> = defineMessages({
	locked: {
		id: 'admin.settings.ports.locked-hint',
		defaultMessage: 'Users cannot add, remove, rename or swap ports. Each server keeps the ports it has.',
	},
	pool: {
		id: 'admin.settings.ports.pool-hint',
		defaultMessage: 'Users can add more ports, for a map or a voice chat; the panel picks the next free one.',
	},
	choose: {
		id: 'admin.settings.ports.choose-hint',
		defaultMessage: 'Users can also type the port number they want, as long as it is free.',
	},
})
</script>
