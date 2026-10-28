<template>
	<footer
		class="flex flex-wrap items-center justify-center gap-x-2 gap-y-1 px-4 py-4 text-xs text-secondary"
		:class="{ 'max-md:pb-20': onServerPage }"
	>
		<span>CraftPanel</span>
		<span aria-hidden="true">·</span>
		<a
			:href="SOURCE_URL"
			target="_blank"
			rel="noopener noreferrer"
			class="text-secondary hover:underline"
		>
			{{ formatMessage(messages.sourceCode) }}
		</a>
		<span aria-hidden="true">·</span>
		<a
			:href="LICENSE_URL"
			target="_blank"
			rel="noopener noreferrer"
			class="text-secondary hover:underline"
		>
			AGPL-3.0
		</a>
	</footer>
</template>

<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import { LICENSE_URL, SOURCE_URL } from '@/source'

const { formatMessage } = useVIntl()
const route = useRoute()

const onServerPage = computed(() => route.params.id !== undefined)

const messages = defineMessages({
	sourceCode: {
		id: 'panel.footer.source-code',
		defaultMessage: 'Source code',
	},
})
</script>
