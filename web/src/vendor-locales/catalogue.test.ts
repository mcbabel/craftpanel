import { readdirSync, readFileSync } from 'node:fs'
import { join, relative, resolve } from 'node:path'

import { IntlMessageFormat } from 'intl-messageformat'
import ts from 'typescript'
import { describe, expect, it } from 'vitest'

const VENDOR = resolve(import.meta.dirname, '../../../vendor/modrinth/ui/src')
const PREFIX = 'craftpanel.vendor.'
const TRANSLATIONS = ['de-DE']

const TEXT = 0
const HASH = 7
const TAG = 8

type Part = {
	type: number
	value?: string
	options?: Record<string, { value: Part[] }>
	children?: Part[]
}

function files(folder: string): string[] {
	return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
		const path = join(folder, entry.name)
		if (entry.isDirectory()) return files(path)
		return /\.(ts|vue)$/.test(entry.name) ? [path] : []
	})
}

function scripts(path: string): string[] {
	const text = readFileSync(path, 'utf8')
	if (!path.endsWith('.vue')) return [text]
	return [...text.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/g)].map((hit) => hit[1])
}

function field(node: ts.ObjectLiteralExpression, name: string): ts.Expression | null {
	for (const property of node.properties) {
		if (!ts.isPropertyAssignment(property)) continue
		const key = property.name
		if ((ts.isIdentifier(key) || ts.isStringLiteralLike(key)) && key.text === name) {
			return property.initializer
		}
	}
	return null
}

function fromTheVendor(): Map<string, { file: string; english: string }> {
	const found = new Map<string, { file: string; english: string }>()
	for (const path of files(VENDOR)) {
		for (const code of scripts(path)) {
			if (!code.includes(PREFIX)) continue
			const tree = ts.createSourceFile(path, code, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
			const walk = (node: ts.Node) => {
				if (ts.isObjectLiteralExpression(node)) {
					const id = field(node, 'id')
					const english = field(node, 'defaultMessage')
					if (
						id !== null &&
						english !== null &&
						ts.isStringLiteralLike(id) &&
						ts.isStringLiteralLike(english) &&
						id.text.startsWith(PREFIX)
					) {
						found.set(id.text, { file: relative(VENDOR, path), english: english.text })
					}
				}
				ts.forEachChild(node, walk)
			}
			walk(tree)
		}
	}
	return found
}

function placeholders(text: string, language: string): string[] {
	const found = new Set<string>()
	const walk = (parts: Part[]) => {
		for (const part of parts) {
			if (part.type === TEXT || part.type === HASH) continue
			if (part.type !== TAG && part.value !== undefined) found.add(part.value)
			for (const branch of Object.values(part.options ?? {})) walk(branch.value)
			walk(part.children ?? [])
		}
	}
	walk(new IntlMessageFormat(text, language).getAst() as Part[])
	return [...found].sort()
}

const source = fromTheVendor()

describe('The patched vendor', () => {
	it('carries the messages this catalogue translates', () => {
		expect(source.size).toBeGreaterThan(0)
	})
})

describe.each(TRANSLATIONS)('The vendor translation %s', (language) => {
	const entries: Record<string, { defaultMessage: string }> = JSON.parse(
		readFileSync(join(import.meta.dirname, language, 'index.json'), 'utf8'),
	)

	it('translates every message the vendor patch put in', () => {
		const missing = [...source.entries()]
			.filter(([id]) => !(id in entries))
			.map(([id, message]) => `${id} (${message.file})`)
			.sort()
		expect(missing).toEqual([])
	})

	it('carries no id the vendor no longer has', () => {
		expect(Object.keys(entries).filter((id) => !source.has(id))).toEqual([])
	})

	it('sets the same placeholders as the English text', () => {
		const different = [...source.entries()]
			.filter(([id]) => id in entries)
			.map(([id, message]) => ({
				id,
				here: placeholders(entries[id].defaultMessage, language),
				there: placeholders(message.english, 'en-US'),
			}))
			.filter(({ here, there }) => here.join(' ') !== there.join(' '))
			.map(({ id, here, there }) => `${id}: ${there.join(', ')} → ${here.join(', ')}`)
		expect(different).toEqual([])
	})
})
