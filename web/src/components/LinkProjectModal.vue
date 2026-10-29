<template>
	<NewModal ref="modal" :header="formatMessage(messages.title)" width="560px">
		<form class="flex w-full flex-col gap-3" @submit.prevent="() => void confirm()">
			<span>{{ formatMessage(messages.body, { file: target?.fileName ?? '' }) }}</span>
			<ProjectCombobox
				v-model="projectId"
				:project-types="projectTypes"
				:limit="20"
				clearable
			/>
			<span class="text-sm text-secondary">{{ formatMessage(messages.hint) }}</span>
			<div class="mb-1 mt-2 flex justify-end gap-2.5">
				<Button @click="modal?.hide()">
					<XIcon aria-hidden="true" />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button type="colored" color="brand" native-type="submit" :disabled="!projectId || busy">
					<LinkIcon aria-hidden="true" />
					{{ formatMessage(messages.confirm) }}
				</Button>
			</div>
		</form>
	</NewModal>
</template>

<script setup lang="ts">
import { LinkIcon, XIcon } from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NewModal,
	ProjectCombobox,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { api, type Ulid } from '@/api'
import type { LinkTarget } from '@/providers/content-manager'

const props = defineProps<{ serverId: Ulid }>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()

const modal = ref<InstanceType<typeof NewModal>>()
const target = ref<LinkTarget | null>(null)
const projectId = ref<string | undefined>(undefined)
const busy = ref(false)

const projectTypes = computed(() => {
	const kind = target.value?.projectType
	if (kind === 'plugin') return ['plugin' as const]
	if (kind === 'mod') return ['mod' as const]
	return kind ? [kind] : undefined
})

const messages = defineMessages({
	title: { id: 'craftpanel.content.link.title', defaultMessage: 'Link to a Modrinth project' },
	body: {
		id: 'craftpanel.content.link.body',
		defaultMessage: 'Modrinth did not recognise {file}. Pick the project it belongs to.',
	},
	hint: {
		id: 'craftpanel.content.link.hint',
		defaultMessage:
			'The file stays as it is. The panel looks for the version that matches its name and checks for updates from then on.',
	},
	confirm: { id: 'craftpanel.content.link.confirm', defaultMessage: 'Link' },
	linked: { id: 'craftpanel.content.link.linked', defaultMessage: 'Linked to Modrinth' },
	linkedNoVersion: {
		id: 'craftpanel.content.link.linked-no-version',
		defaultMessage:
			'Linked to Modrinth. No version matched the file name, so pick one under Update to replace the file.',
	},
	failed: { id: 'craftpanel.content.link.failed', defaultMessage: 'The file could not be linked' },
})

function open(next: LinkTarget): void {
	target.value = next
	projectId.value = undefined
	modal.value?.show()
}

async function confirm(): Promise<void> {
	const item = target.value
	if (!item || !projectId.value) return
	busy.value = true
	try {
		const linked = await api.content.link(props.serverId, { id: item.id, project_id: projectId.value })
		addNotification({
			type: 'success',
			title: formatMessage(linked.version_id ? messages.linked : messages.linkedNoVersion),
		})
		modal.value?.hide()
	} catch (cause) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.failed),
			text: cause instanceof Error ? cause.message : undefined,
		})
	} finally {
		busy.value = false
	}
}

defineExpose({ open })
</script>
