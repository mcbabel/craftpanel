import { describe, expect, it } from 'vitest'

import { withoutProfileLinks } from './profile-links'

describe('an audit entry about a user', () => {
	it('loses the link to a profile page, which would lead to modrinth.com', () => {
		const event = {
			key: 'user_invited',
			props: {
				action: 'user_invited',
				targetUser: { id: 'u1', label: 'anna', to: '/user/anna', iconShape: 'circle' },
				permissions: 'BASE_READ',
			},
		}
		const cleaned = withoutProfileLinks(event)
		expect(cleaned.props.targetUser).toEqual({ id: 'u1', label: 'anna', iconShape: 'circle' })
		expect(cleaned.props.permissions).toBe('BASE_READ')
		expect(event.props.targetUser.to, 'the parsed entry itself stays as it was').toBe('/user/anna')
	})

	it('keeps links that stay inside the panel', () => {
		const backup = { id: 'b1', label: 'Before update', to: '/servers/s1/backups' }
		const cleaned = withoutProfileLinks({ props: { backups: [backup] } })
		expect(cleaned.props.backups).toEqual([backup])
	})
})
