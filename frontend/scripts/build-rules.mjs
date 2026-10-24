import { copyFileSync, mkdirSync, statSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'

const frontendRoot=resolve(dirname(fileURLToPath(import.meta.url)),'..')
const backendRoot=resolve(frontendRoot,'../backend')
const manifest=resolve(backendRoot,'game-rules/Cargo.toml')
const result=spawnSync('cargo',['build','--manifest-path',manifest,'--target','wasm32-unknown-unknown','--release','--features','wasm'],{cwd:backendRoot,stdio:'inherit',shell:process.platform==='win32'})
if(result.status!==0)process.exit(result.status||1)
const source=resolve(backendRoot,'target/wasm32-unknown-unknown/release/levant_score_rules.wasm')
const target=resolve(frontendRoot,'public/game_rules.wasm')
mkdirSync(dirname(target),{recursive:true})
copyFileSync(source,target)
console.log(`Wrote ${target} (${statSync(target).size} bytes)`)

