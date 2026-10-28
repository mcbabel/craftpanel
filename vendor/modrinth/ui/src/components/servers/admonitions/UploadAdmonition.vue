<template>
	<Admonition type="info" :progress="overallProgress" progress-color="blue">
		<template #icon>
			<UploadIcon class="h-6 w-6 flex-none text-brand-blue" />
		</template>
		<template #header>
			{{
				state.currentFileName
					? formatMessage(messages.uploadingFile, {
							name: state.currentFileName,
							completed: state.completedFiles,
							total: state.totalFiles,
						})
					: formatMessage(messages.uploadingFiles, {
							completed: state.completedFiles,
							total: state.totalFiles,
						})
			}}
		</template>
		<span class="text-secondary">
			{{ formatBytes(state.uploadedBytes) }} / {{ formatBytes(state.totalBytes) }} ({{
				Math.round(overallProgress * 100)
			}}%)
		</span>
		<template v-if="cancelable" #top-right-actions>
			<Button
				type="outlined"
				class="!border !text-blue [&>svg]:!text-blue !shadow-[inset_0_0_0_1px_var(--color-blue)]"
				native-type="button"
				:disabled="cancelling"
				@click="$emit('cancel')"
			>
				{{ formatMessage(commonMessages.cancelButton) }}
			</Button>
		</template>
	</Admonition>
</template>

<script setup lang="ts">
import { UploadIcon } from '@modrinth/assets'
import { computed } from 'vue'

import Admonition from '#ui/components/base/Admonition.vue'
import { Button } from '#ui/components/base/buttons'
import { useFormatBytes } from '#ui/composables'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { injectModrinthServerContext } from '#ui/providers'
import { commonMessages } from '#ui/utils/common-messages'

withDefaults(
	defineProps<{
		cancelable?: boolean
		cancelling?: boolean
	}>(),
	{
		cancelable: true,
		cancelling: false,
	},
)

defineEmits<{
	cancel: []
}>()

const formatBytes = useFormatBytes()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	uploadingFile: {
		id: 'craftpanel.vendor.upload.uploading-file',
		defaultMessage: 'Uploading {name} ({completed}/{total})',
	},
	uploadingFiles: {
		id: 'craftpanel.vendor.upload.uploading-files',
		defaultMessage: 'Uploading files ({completed}/{total})',
	},
})

const ctx = injectModrinthServerContext()

const state = computed(() => ctx.uploadState.value)

const overallProgress = computed(() => {
	const s = state.value
	if (!s.isUploading || s.totalFiles === 0) return 0
	return Math.min((s.completedFiles + s.currentFileProgress) / s.totalFiles, 1)
})
</script>
