import Context from "https://deno.land/std@0.108.0/wasi/snapshot_preview1.ts";

const context = new Context({
  preopens: {
    '/': './fs',
  },
})

const binary = await Deno.readFile("../target/wasm32-wasi/release/wasm.wasm")
const module = await WebAssembly.compile(binary)
const instance = await WebAssembly.instantiate(module, {
  wasi_snapshot_preview1: context.exports,
})

const { run_go, alloc, dealloc} = instance.exports as Record<string, CallableFunction>
const { memory } = instance.exports as { memory: WebAssembly.Memory }
context.start(instance)

const path = '/fibonacci.gos'

const strBuf = new TextEncoder().encode(path)
const ptr = alloc(strBuf.byteLength)
const memBuf = new Uint8Array(memory.buffer, ptr, strBuf.length)

memBuf.set(strBuf)

const exitCode = run_go(ptr)
console.log(`Running Go exited with code: ${exitCode}`)




