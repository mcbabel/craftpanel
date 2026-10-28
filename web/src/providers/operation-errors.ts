import { defineMessages } from '@modrinth/ui/src/composables/i18n.ts'

import type { OperationError } from '@/api'

const messages = defineMessages({
	helperUnavailable: {
		id: 'craftpanel.operations.helper-unavailable',
		defaultMessage:
			"The panel's helper service is not running, so this could not be done. Check craftpanel-helper.service on the machine.",
	},
})

const TRANSLATED: Record<string, { id: string; defaultMessage: string }> = {
	helper_unavailable: messages.helperUnavailable,
}

let translate: (id: string) => string | null = () => null

export function describeOperationErrors(lookup: (id: string) => string): void {
	translate = lookup
}

export function knownErrorText(code: string | null | undefined): string | undefined {
	const known = code ? TRANSLATED[code] : undefined
	return known ? (translate(known.id) ?? known.defaultMessage) : undefined
}

export function operationErrorText(error: OperationError | null | undefined): string | undefined {
	if (!error) return undefined
	return knownErrorText(error.code) ?? error.message
}
