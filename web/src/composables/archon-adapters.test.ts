import { describe, expect, it, vi } from 'vitest'

import type { BackupSchedule } from '@/api'

vi.mock('@modrinth/ui', () => ({
	injectModrinthClient: () => ({}),
	provideModrinthClient: () => undefined,
}))

const { toOptions } = await import('./archon-adapters')

function schedule(limit: number, automatic: number): BackupSchedule {
	return {
		limit,
		automatic_slots: automatic,
		manual_slots: limit - automatic,
		automatic_used: 0,
		manual_used: 0,
		interval_hours: 12,
		hour_utc: 4,
		next_run_at: null,
		last_run_at: null,
		last_status: null,
		last_error: null,
	}
}

describe('The backup slots in the shape of the settings tab', () => {
	it('splits the whole quota when it fits the tab', () => {
		expect(toOptions(schedule(10, 4))).toEqual({
			limit: 10,
			split_automatic: 4,
			split_manual: 6,
			interval: 12,
		})
	})

	it('keeps the interval while automatic backups are off', () => {
		expect(toOptions(schedule(10, 0))).toMatchObject({ split_automatic: 0, interval: 12 })
	})

	it('shows a larger quota as a window the slider can hold, and the two halves still add up', () => {
		const options = toOptions(schedule(50, 5))
		expect(options.limit).toBe(15)
		expect(options.split_automatic + options.split_manual).toBe(options.limit)
		expect(options.split_automatic).toBe(5)
	})
})
