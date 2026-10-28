<template>
	<div class="flex flex-col gap-6 max-md:gap-4">
		<ServerManageStats
			class="server-stats"
			:data="isConnected ? stats : undefined"
			:loading="!isConnected"
		/>

		<PlayitAddress />

		<div class="server-console flex min-h-[700px] flex-col gap-2 max-md:min-h-[520px]">
			<h2 class="m-0 shrink-0 text-2xl font-semibold text-contrast">
				{{ formatMessage(messages.console) }}
			</h2>
			<ConsolePageLayout />
		</div>
	</div>
</template>

<script setup lang="ts">
import { ConsolePageLayout, defineMessages, useVIntl } from '@modrinth/ui'
import ServerManageStats from '@modrinth/ui/src/components/servers/ServerManageStats.vue'

import PlayitAddress from '@/components/PlayitAddress.vue'
import { useServerPage } from '@/composables/server-page'

const { formatMessage } = useVIntl()
const { context } = useServerPage()
const { isConnected, stats } = context

const messages = defineMessages({
	console: {
		id: 'panel.server.console',
		defaultMessage: 'Console',
	},
})
</script>

<style scoped>
@media (max-width: 767px) {
	.server-stats {
		flex-direction: row;
		align-items: stretch;
		gap: 0.5rem;
	}

	.server-stats > :deep(*) {
		min-width: 0;
		min-height: 0;
		flex: 1 1 0;
		border-radius: 14px;
		padding: 0.625rem 0.75rem;
	}

	.server-stats :deep(.text-lg) {
		font-size: 0.75rem;
		line-height: 1rem;
	}

	.server-stats :deep(.size-6) {
		width: 1rem;
		height: 1rem;
	}

	.server-stats :deep(.text-3xl) {
		font-size: 1.05rem;
		line-height: 1.4rem;
	}

	.server-stats :deep(.chart-space) {
		display: none;
	}

	.server-console :deep(.flex.flex-col.gap-2 > .flex.items-center.gap-2) {
		flex-wrap: wrap;
	}

	.server-console :deep(.flex.flex-col.gap-2 > .flex.items-center.gap-2 > .w-\[220px\]) {
		width: 100%;
	}

	.server-console :deep(.flex.flex-col.gap-2 > .flex.items-center.justify-between) {
		flex-wrap: wrap;
		gap: 0.5rem;
	}
}
</style>
