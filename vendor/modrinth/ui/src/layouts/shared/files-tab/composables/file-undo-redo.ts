import { ref } from 'vue'

import { defineMessages, useVIntl } from '../../../../composables/i18n'
import type { Operation } from '../types'

const messages = defineMessages({
	undoneTitle: {
		id: 'craftpanel.vendor.files.undone-title',
		defaultMessage: '{type, select, move {Move} other {Rename}} undone',
	},
	undoneText: {
		id: 'craftpanel.vendor.files.undone-text',
		defaultMessage:
			'{fileName} has been restored to its original {type, select, move {location} other {name}}',
	},
	undoFailedTitle: {
		id: 'craftpanel.vendor.files.undo-failed-title',
		defaultMessage: 'Undo failed',
	},
	undoFailedText: {
		id: 'craftpanel.vendor.files.undo-failed-text',
		defaultMessage: 'Failed to undo the last {type, select, move {move} other {rename}} operation',
	},
	redoneTitle: {
		id: 'craftpanel.vendor.files.redone-title',
		defaultMessage: '{type, select, move {Move} other {Rename}} redone',
	},
	redoneText: {
		id: 'craftpanel.vendor.files.redone-text',
		defaultMessage: '{fileName} has been {type, select, move {moved} other {renamed}} again',
	},
	redoFailedTitle: {
		id: 'craftpanel.vendor.files.redo-failed-title',
		defaultMessage: 'Redo failed',
	},
	redoFailedText: {
		id: 'craftpanel.vendor.files.redo-failed-text',
		defaultMessage: 'Failed to redo the last {type, select, move {move} other {rename}} operation',
	},
})

export function useFileUndoRedo(
	renameItem: (path: string, newName: string) => Promise<void>,
	moveItem: (source: string, destination: string) => Promise<void>,
	refresh: () => void,
	notify: (title: string, text: string, type: 'success' | 'error') => void,
) {
	const { formatMessage } = useVIntl()
	const operationHistory = ref<Operation[]>([])
	const redoStack = ref<Operation[]>([])

	function recordOperation(op: Operation) {
		redoStack.value = []
		operationHistory.value.push(op)
	}

	async function undo() {
		const lastOperation = operationHistory.value.pop()
		if (!lastOperation) return

		try {
			switch (lastOperation.type) {
				case 'move':
					await moveItem(
						`${lastOperation.destinationPath}/${lastOperation.fileName}`.replace('//', '/'),
						`${lastOperation.sourcePath}/${lastOperation.fileName}`.replace('//', '/'),
					)
					break
				case 'rename':
					await renameItem(
						`${lastOperation.path}/${lastOperation.newName}`.replace('//', '/'),
						lastOperation.oldName,
					)
					break
			}

			redoStack.value.push(lastOperation)
			refresh()
			notify(
				formatMessage(messages.undoneTitle, { type: lastOperation.type }),
				formatMessage(messages.undoneText, {
					fileName: lastOperation.fileName,
					type: lastOperation.type,
				}),
				'success',
			)
		} catch {
			notify(
				formatMessage(messages.undoFailedTitle),
				formatMessage(messages.undoFailedText, { type: lastOperation.type }),
				'error',
			)
		}
	}

	async function redo() {
		const lastOperation = redoStack.value.pop()
		if (!lastOperation) return

		try {
			switch (lastOperation.type) {
				case 'move':
					await moveItem(
						`${lastOperation.sourcePath}/${lastOperation.fileName}`.replace('//', '/'),
						`${lastOperation.destinationPath}/${lastOperation.fileName}`.replace('//', '/'),
					)
					break
				case 'rename':
					await renameItem(
						`${lastOperation.path}/${lastOperation.oldName}`.replace('//', '/'),
						lastOperation.newName,
					)
					break
			}

			operationHistory.value.push(lastOperation)
			refresh()
			notify(
				formatMessage(messages.redoneTitle, { type: lastOperation.type }),
				formatMessage(messages.redoneText, {
					fileName: lastOperation.fileName,
					type: lastOperation.type,
				}),
				'success',
			)
		} catch {
			notify(
				formatMessage(messages.redoFailedTitle),
				formatMessage(messages.redoFailedText, { type: lastOperation.type }),
				'error',
			)
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === 'z') {
			e.preventDefault()
			undo()
		}
		if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'z') {
			e.preventDefault()
			redo()
		}
	}

	return {
		operationHistory,
		redoStack,
		recordOperation,
		undo,
		redo,
		onKeydown,
	}
}
