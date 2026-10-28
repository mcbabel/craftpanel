import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const source = join(root, '..', 'docs')
const target = join(root, 'docs')
const repository = 'https://github.com/MCbabel/craftpanel/blob/main'

function relink(markdown) {
	return markdown
		.replace(/\]\(README\.md(#[^)]*)?\)/g, (_, anchor = '') => `](./${anchor})`)
		.replace(/\]\(\.\.\/([^)]+)\)/g, (_, path) => `](${repository}/${path})`)
}

rmSync(target, { recursive: true, force: true })
mkdirSync(target, { recursive: true })
for (const name of readdirSync(source)) {
	if (!name.endsWith('.md')) continue
	const text = relink(readFileSync(join(source, name), 'utf8'))
	writeFileSync(join(target, name === 'README.md' ? 'index.md' : name), text)
}
