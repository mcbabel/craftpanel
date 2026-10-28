<template>
	<div class="flex min-h-full flex-col">
		<header class="border-0 border-b border-solid border-divider bg-bg-raised">
			<div class="mx-auto flex h-16 w-full max-w-[1280px] items-center gap-4 px-4 sm:px-6">
				<RouterLink
					:to="{ name: 'servers' }"
					class="flex items-center gap-2 text-lg font-extrabold text-contrast no-underline"
				>
					<img v-if="branding.logo_url" :src="branding.logo_url" alt="" class="h-7 w-auto max-w-[7rem] object-contain" />
					<CraftPanelLogo v-else class="h-7 w-auto" />
					{{ branding.name }}
				</RouterLink>

				<TeleportOverflowMenu
					class="ml-auto !gap-1 !rounded-xl !px-2"
					type="quiet"
					size="lg"
					interaction="none"
					:icon-only="false"
					:label="menuLabel"
					:options="options"
				>
					<span class="relative inline-flex">
						<Avatar :src="user?.avatar_url" size="1.75rem" circle />
						<span
							v-if="invitationCount > 0"
							class="absolute -right-1.5 -top-1.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-brand px-1 text-[0.625rem] font-bold leading-none text-brand-inverted"
							aria-hidden="true"
						>
							{{ invitationCount }}
						</span>
					</span>
					<span class="max-sm:hidden">{{ username }}</span>
					<DropdownIcon class="size-5 text-secondary" />
				</TeleportOverflowMenu>
			</div>
		</header>

		<main class="mx-auto w-full max-w-[1280px] flex-1 px-4 py-4 sm:px-6 sm:py-6">
			<RouterView />
		</main>

		<NewModal ref="preferencesModal" :header="formatMessage(messages.preferences)" width="44rem">
			<div class="flex flex-col gap-6">
				<section class="flex flex-col gap-3">
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(commonSettingsMessages.appearance) }}
					</h3>
					<ThemeSelector
						:aria-label="formatMessage(commonSettingsMessages.appearance)"
						:model-value="choice"
						:theme-options="themeChoices"
						:system-theme-color="systemTheme"
						preferred-dark-theme="dark"
						@update:model-value="chooseTheme"
					/>
				</section>
				<section class="flex flex-col gap-3">
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(commonSettingsMessages.language) }}
					</h3>
					<LanguageSelector
						product="website"
						:current-locale="locale"
						:locales="locales"
						:on-locale-change="changeLocale"
					/>
				</section>
			</div>
		</NewModal>

		<NotificationPanel />
	</div>
</template>

<script setup lang="ts">
import {
	CircleUserIcon,
	DropdownIcon,
	LogOutIcon,
	MailIcon,
	PaletteIcon,
	ServerStackIcon,
	ShieldCheckIcon,
} from '@modrinth/assets'
import {
	Avatar,
	type ButtonMenuOption,
	commonMessages,
	commonSettingsMessages,
	defineMessages,
	LOCALES,
	NewModal,
	NotificationPanel,
	TeleportOverflowMenu,
	useVIntl,
} from '@modrinth/ui'
import ThemeSelector from '@modrinth/ui/src/layouts/shared/appearance-settings/components/appearance-settings-theme-selector.vue'
import LanguageSelector from '@modrinth/ui/src/layouts/wrapped/settings/language-settings/language-settings-selector.vue'
import { computed, onMounted, onScopeDispose, ref } from 'vue'
import { RouterLink, RouterView, useRouter } from 'vue-router'

import { setCopyFailureHandler } from '@/clipboard'
import CraftPanelLogo from '@/components/CraftPanelLogo.vue'
import { useBranding } from '@/composables/branding'
import { isLocale, useLocale } from '@/composables/locale'
import { useInvitations } from '@/composables/invitations'
import { provideModrinthHost, provideNotifications } from '@/composables/modrinth-host'
import { useSession } from '@/composables/session'
import { isChoice, themeChoices, useTheme } from '@/composables/theme'

const { formatMessage } = useVIntl()
const router = useRouter()
const { branding } = useBranding()
const { user, isAdmin, signOut } = useSession()
const { choice, systemTheme, setTheme } = useTheme()
const { locale, setLocale } = useLocale()

const notifications = provideNotifications()
provideModrinthHost()
const invitations = useInvitations()
const invitationCount = invitations.count

const preferencesModal = ref<InstanceType<typeof NewModal> | null>(null)

const locales = LOCALES.filter((entry) => isLocale(entry.code))

function chooseTheme(next: string): void {
	if (isChoice(next)) setTheme(next)
}

function changeLocale(next: string): Promise<void> {
	setLocale(next)
	return Promise.resolve()
}

const messages = defineMessages({
	account: {
		id: 'panel.menu.account',
		defaultMessage: 'Your account',
	},
	preferences: {
		id: 'panel.menu.preferences',
		defaultMessage: 'Appearance and language',
	},
	administration: {
		id: 'panel.menu.administration',
		defaultMessage: 'Administration',
	},
	copyFailed: {
		id: 'panel.copy.failed',
		defaultMessage: 'Nothing was copied',
	},
	invitations: {
		id: 'panel.menu.invitations',
		defaultMessage: '{count, plural, one {# invitation} other {# invitations}}',
	},
	menuLabel: {
		id: 'panel.menu.label-with-invitations',
		defaultMessage: '{name}, {count, plural, one {# open invitation} other {# open invitations}}',
	},
	copyFailedHint: {
		id: 'panel.copy.failed-hint',
		defaultMessage:
			'This browser would not put the text on the clipboard. Select it and copy it by hand.',
	},
})

setCopyFailureHandler(() =>
	notifications.addNotification({
		title: formatMessage(messages.copyFailed),
		text: formatMessage(messages.copyFailedHint),
		type: 'error',
	}),
)
onScopeDispose(() => setCopyFailureHandler(null))

const username = computed(() => user.value?.username ?? '')
const menuLabel = computed(() =>
	invitationCount.value > 0
		? formatMessage(messages.menuLabel, { name: username.value, count: invitationCount.value })
		: username.value,
)

function refreshInvitations(): void {
	if (document.visibilityState === 'visible') void invitations.refresh()
}

onMounted(() => {
	void invitations.refresh()
	window.addEventListener('focus', refreshInvitations)
})
onScopeDispose(() => window.removeEventListener('focus', refreshInvitations))

const options = computed<ButtonMenuOption[]>(() => [
	{
		id: 'invitations',
		type: 'link',
		to: { name: 'servers' },
		label: formatMessage(messages.invitations, { count: invitationCount.value }),
		icon: MailIcon,
		tone: 'brand',
		shown: invitationCount.value > 0,
	},
	{
		id: 'servers',
		type: 'link',
		to: { name: 'servers' },
		label: formatMessage(commonMessages.serversLabel),
		icon: ServerStackIcon,
	},
	{
		id: 'account',
		type: 'link',
		to: { name: 'account' },
		label: formatMessage(messages.account),
		icon: CircleUserIcon,
	},
	{
		id: 'preferences',
		label: formatMessage(messages.preferences),
		icon: PaletteIcon,
		action: () => preferencesModal.value?.show(),
	},
	{ type: 'divider', shown: isAdmin.value },
	{
		id: 'administration',
		type: 'link',
		to: { name: 'admin-overview' },
		label: formatMessage(messages.administration),
		icon: ShieldCheckIcon,
		shown: isAdmin.value,
	},
	{ type: 'divider' },
	{
		id: 'sign-out',
		label: formatMessage(commonMessages.signOutButton),
		icon: LogOutIcon,
		tone: 'red',
		hoverFilled: true,
		action: () => void leave(),
	},
])

async function leave() {
	await signOut()
	invitations.forget()
	await router.push({ name: 'login' })
}
</script>
