import { defineMessages } from '@modrinth/ui/src/composables/i18n.ts'

import type { OperationError } from '@/api'

const messages = defineMessages({
	helperUnavailable: {
		id: 'craftpanel.operations.helper-unavailable',
		defaultMessage:
			"The panel's helper service is not running, so the files could not be handed to the server. Check craftpanel-helper.service on the machine.",
	},
})

const TRANSLATED: Record<string, { id: string; defaultMessage: string }> = {
	helper_unavailable: messages.helperUnavailable,
}

// Set by the i18n plugin when it starts, so this module stays free of the app's setup.
let translate: (id: string) => string | null = () => null

export function describeOperationErrors(lookup: (id: string) => string): void {
	translate = lookup
}

// The panel words its errors in English. Where the reader can act on one, it gets a sentence in
// their language instead.
export function operationErrorText(error: OperationError | null | undefined): string | undefined {
	if (!error) return undefined
	const known = TRANSLATED[error.code]
	return known ? (translate(known.id) ?? known.defaultMessage) : error.message
}
