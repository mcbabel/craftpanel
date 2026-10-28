<template>
	<ShieldCheckIcon
		v-if="role === 'admin'"
		v-tooltip="formatMessage(messages.adminLabel)"
		role="img"
		class="size-4 shrink-0 !text-green"
	/>
	<ScaleIcon
		v-else-if="role === 'moderator'"
		v-tooltip="formatMessage(messages.moderatorLabel)"
		role="img"
		class="size-4 shrink-0 !text-orange"
	/>
</template>

<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import { ScaleIcon, ShieldCheckIcon } from '@modrinth/assets'

import { defineMessages, useVIntl } from '#ui/composables/i18n'

defineProps<{
	role?: Labrinth.Users.v2.Role | null
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	adminLabel: {
		id: 'user.role.admin',
		defaultMessage: 'Admin',
	},
	moderatorLabel: {
		id: 'user.role.moderator',
		defaultMessage: 'Content Moderator',
	},
})
</script>
