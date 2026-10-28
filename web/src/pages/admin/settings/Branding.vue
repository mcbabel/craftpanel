<template>
	<Card class="!mb-0 flex flex-col gap-6">
		<SettingsLabel :title="formatMessage(messages.title)" :description="formatMessage(messages.description)" />

		<div class="flex flex-col gap-1.5">
			<label class="text-sm font-semibold text-contrast" for="brand-name">
				{{ formatMessage(messages.name) }}
			</label>
			<div class="flex flex-wrap items-center gap-2">
				<Input
					id="brand-name"
					v-model="name"
					:maxlength="40"
					placeholder="CraftPanel"
					wrapper-class="w-full sm:w-72"
				/>
				<Button :disabled="busy || !nameChanged" @click="() => void saveName()">
					<SaveIcon aria-hidden="true" />
					{{ formatMessage(commonMessages.saveButton) }}
				</Button>
			</div>
			<span class="text-xs text-secondary">{{ formatMessage(messages.nameHint) }}</span>
		</div>

		<div class="flex flex-col gap-3">
			<span class="text-sm font-semibold text-contrast">{{ formatMessage(messages.logo) }}</span>
			<div class="flex flex-wrap items-center gap-4">
				<div class="flex size-16 items-center justify-center rounded-2xl border border-solid border-surface-5 bg-surface-2 p-2">
					<img v-if="branding.logo_url" :src="branding.logo_url" alt="" class="max-h-full max-w-full object-contain" />
					<CraftPanelLogo v-else class="h-full w-auto" />
				</div>
				<div class="flex flex-wrap gap-2">
					<Button :disabled="busy" @click="picker?.click()">
						<UploadIcon aria-hidden="true" />
						{{ formatMessage(branding.logo_url ? messages.replaceLogo : messages.uploadLogo) }}
					</Button>
					<Button v-if="branding.logo_url" :disabled="busy" @click="() => void removeLogo()">
						<TrashIcon aria-hidden="true" />
						{{ formatMessage(messages.removeLogo) }}
					</Button>
				</div>
				<input
					ref="picker"
					type="file"
					accept="image/png,image/jpeg,image/webp"
					class="hidden"
					@change="(event) => void pick(event)"
				/>
			</div>
			<span class="text-xs text-secondary">{{ formatMessage(messages.logoHint) }}</span>
		</div>
	</Card>
</template>

<script setup lang="ts">
import { SaveIcon, TrashIcon, UploadIcon } from '@modrinth/assets'
import {
	Button,
	Card,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	Input,
	SettingsLabel,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { api, type BrandingView, isApiRequestError } from '@/api'
import CraftPanelLogo from '@/components/CraftPanelLogo.vue'
import { useBranding } from '@/composables/branding'

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const { branding, adopt, refresh } = useBranding()

const picker = ref<HTMLInputElement | null>(null)
const busy = ref(false)
const name = ref(branding.value.custom_name ? branding.value.name : '')

watch(
	() => branding.value,
	(view) => {
		if (!busy.value) name.value = view.custom_name ? view.name : ''
	},
)

const nameChanged = computed(() => {
	const saved = branding.value.custom_name ? branding.value.name : ''
	return name.value.trim() !== saved
})

const messages = defineMessages({
	title: { id: 'admin.branding.title', defaultMessage: 'Branding' },
	description: {
		id: 'admin.branding.description',
		defaultMessage:
			'Your own name and logo for the panel. They show in the header, the browser tab and the mails it sends.',
	},
	name: { id: 'admin.branding.name', defaultMessage: 'Name' },
	nameHint: {
		id: 'admin.branding.name-hint',
		defaultMessage: 'Leave it empty to go back to CraftPanel. At most 40 characters.',
	},
	logo: { id: 'admin.branding.logo', defaultMessage: 'Logo' },
	uploadLogo: { id: 'admin.branding.upload-logo', defaultMessage: 'Upload logo' },
	replaceLogo: { id: 'admin.branding.replace-logo', defaultMessage: 'Replace logo' },
	removeLogo: { id: 'admin.branding.remove-logo', defaultMessage: 'Use the CraftPanel logo' },
	logoHint: {
		id: 'admin.branding.logo-hint',
		defaultMessage:
			'PNG, JPEG or WebP, at most 512 KiB, ideally square. Mails only carry PNG and JPEG; with a WebP logo they keep the CraftPanel logo.',
	},
	saved: { id: 'admin.branding.saved', defaultMessage: 'Branding saved' },
	failed: { id: 'admin.branding.failed', defaultMessage: 'The branding could not be changed' },
})

async function run(change: () => Promise<BrandingView>): Promise<void> {
	busy.value = true
	try {
		adopt(await change())
		addNotification({ type: 'success', title: formatMessage(messages.saved) })
	} catch (error) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.failed),
			text: isApiRequestError(error) || error instanceof Error ? error.message : undefined,
		})
		await refresh()
	} finally {
		busy.value = false
	}
}

async function saveName(): Promise<void> {
	const wanted = name.value.trim()
	await run(() => api.branding.rename(wanted === '' ? null : wanted))
}

async function pick(event: Event): Promise<void> {
	const input = event.target as HTMLInputElement
	const file = input.files?.[0]
	input.value = ''
	if (!file) return
	await run(() => api.branding.uploadLogo(file))
}

async function removeLogo(): Promise<void> {
	await run(() => api.branding.removeLogo())
}
</script>
