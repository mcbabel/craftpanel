export interface NewServerDraft {
	name: string
	memory: number
	eula: boolean
}

let kept: NewServerDraft | null = null

export function keepDraft(draft: NewServerDraft): void {
	kept = { ...draft }
}

export function takeDraft(): NewServerDraft | null {
	const draft = kept
	kept = null
	return draft
}
