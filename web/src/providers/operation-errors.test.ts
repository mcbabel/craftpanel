import { describe, expect, it } from 'vitest'

import { describeOperationErrors, operationErrorText } from './operation-errors'

describe('operationErrorText', () => {
	it('puts a missing helper into words and leaves other errors as they came', () => {
		const helper = { code: 'helper_unavailable', message: 'raw', step: 'filesystem' } as const
		const other = { code: 'chown_failed', message: 'as it came', step: 'filesystem' } as const

		expect(operationErrorText(helper)).toContain('craftpanel-helper.service')
		expect(operationErrorText(other)).toBe('as it came')
		expect(operationErrorText(null)).toBeUndefined()

		describeOperationErrors((id) => `translated ${id}`)
		expect(operationErrorText(helper)).toBe('translated craftpanel.operations.helper-unavailable')
	})
})
