import { describe, expect, it } from 'vitest'

import { serverAddress } from './address'

describe('serverAddress', () => {
	it('shows what was written on the server, as it was written', () => {
		expect(serverAddress({ ip: 'panel.example', port: 25571, domain: 'mcbabel.de' })).toBe(
			'mcbabel.de',
		)
	})

	it('falls back to the panel address with the port', () => {
		expect(serverAddress({ ip: 'play.example', port: 25563, domain: '' })).toBe(
			'play.example:25563',
		)
	})

	it('invents nothing when neither is set', () => {
		expect(serverAddress({ ip: null, port: 25565, domain: '' })).toBeNull()
		expect(serverAddress(undefined)).toBeNull()
	})
})
