<template>
	<div class="flex gap-1 text-sm font-medium">
		<!-- indicator icon -->

		{{ formatMessage(messages.players, { current: currentPlayers, max: maxPlayers }) }}
	</div>
</template>

<script setup lang="ts">
import { defineMessages, useVIntl } from '../../../composables/i18n'

interface ServerPlayerCountProps {
	currentPlayers: number
	maxPlayers: number
	online: boolean
}

defineProps<ServerPlayerCountProps>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	players: {
		id: 'craftpanel.vendor.server-labels.players',
		defaultMessage: '{current} / {max} Players',
	},
})
</script>
