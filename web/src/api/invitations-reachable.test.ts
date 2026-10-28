import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import { api } from './client'

const ROOT = resolve(import.meta.dirname, '../../..')

function read(path: string): string {
	return readFileSync(resolve(ROOT, path), 'utf8')
}

describe('an invitation', () => {
	it('is fetched, accepted and declined by the client the inbox uses', () => {
		const inbox = read('web/src/composables/invitations.ts')
		for (const call of ['invitations', 'acceptInvitation', 'declineInvitation'] as const) {
			expect(typeof api.access[call], call).toBe('function')
			expect(inbox, `the inbox never makes api.access.${call}`).toContain(`api.access.${call}(`)
		}
	})

	it('is answered by a button a person clicks', () => {
		const card = read('web/src/components/InvitationInbox.vue')
		expect(card).toContain('@click="() => void accept(invitation)"')
		expect(card).toContain('@click="() => void decline(invitation)"')
		expect(card).toContain('await inbox.accept(invitation.id)')
		expect(card).toContain('await inbox.decline(invitation.id)')
	})

	it('shows on the server list, which reloads once one is accepted', () => {
		const list = read('web/src/pages/servers/Index.vue')
		expect(list).toContain('<InvitationInbox @accepted="() => void load(false)" />')
		expect(list).toContain('invitations.refresh()')
	})

	it('is counted in the user menu', () => {
		const shell = read('web/src/layouts/AppShell.vue')
		expect(shell).toContain('void invitations.refresh()')
		expect(shell).toContain("id: 'invitations'")
	})
})

describe('a member on the access page', () => {
	it('is not linked to a profile on modrinth.com', () => {
		const page = read('web/src/pages/servers/Access.vue')
		expect(page).not.toContain(':user-profile-link="() => undefined"')
		expect(page).toContain(':user-profile-link="noProfileLink"')
		expect(page).toContain('withoutProfileLinks(parseAuditEvent(')
	})
})
