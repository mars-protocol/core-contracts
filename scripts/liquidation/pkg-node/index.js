let imports = {}
imports['__wbindgen_placeholder__'] = module.exports
let wasm
const { TextEncoder, TextDecoder } = require(`util`)

const heap = new Array(128).fill(undefined)

heap.push(undefined, null, true, false)

function getObject(idx) {
  return heap[idx]
}

let heap_next = heap.length

function addHeapObject(obj) {
  if (heap_next === heap.length) heap.push(heap.length + 1)
  const idx = heap_next
  heap_next = heap[idx]

  heap[idx] = obj
  return idx
}

function dropObject(idx) {
  if (idx < 132) return
  heap[idx] = heap_next
  heap_next = idx
}

function takeObject(idx) {
  const ret = getObject(idx)
  dropObject(idx)
  return ret
}

let WASM_VECTOR_LEN = 0

let cachedUint8Memory0 = null

function getUint8Memory0() {
  if (cachedUint8Memory0 === null || cachedUint8Memory0.byteLength === 0) {
    cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer)
  }
  return cachedUint8Memory0
}

let cachedTextEncoder = new TextEncoder('utf-8')

const encodeString =
  typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
        return cachedTextEncoder.encodeInto(arg, view)
      }
    : function (arg, view) {
        const buf = cachedTextEncoder.encode(arg)
        view.set(buf)
        return {
          read: arg.length,
          written: buf.length,
        }
      }

function passStringToWasm0(arg, malloc, realloc) {
  if (realloc === undefined) {
    const buf = cachedTextEncoder.encode(arg)
    const ptr = malloc(buf.length, 1) >>> 0
    getUint8Memory0()
      .subarray(ptr, ptr + buf.length)
      .set(buf)
    WASM_VECTOR_LEN = buf.length
    return ptr
  }

  let len = arg.length
  let ptr = malloc(len, 1) >>> 0

  const mem = getUint8Memory0()

  let offset = 0

  for (; offset < len; offset++) {
    const code = arg.charCodeAt(offset)
    if (code > 0x7f) break
    mem[ptr + offset] = code
  }

  if (offset !== len) {
    if (offset !== 0) {
      arg = arg.slice(offset)
    }
    ptr = realloc(ptr, len, (len = offset + arg.length * 3), 1) >>> 0
    const view = getUint8Memory0().subarray(ptr + offset, ptr + len)
    const ret = encodeString(arg, view)

    offset += ret.written
    ptr = realloc(ptr, len, offset, 1) >>> 0
  }

  WASM_VECTOR_LEN = offset
  return ptr
}

function isLikeNone(x) {
  return x === undefined || x === null
}

let cachedInt32Memory0 = null

function getInt32Memory0() {
  if (cachedInt32Memory0 === null || cachedInt32Memory0.byteLength === 0) {
    cachedInt32Memory0 = new Int32Array(wasm.memory.buffer)
  }
  return cachedInt32Memory0
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true })

cachedTextDecoder.decode()

function getStringFromWasm0(ptr, len) {
  ptr = ptr >>> 0
  return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len))
}
/**
 * @param {LiquidationAmountInputs} inputs
 * @returns {LiquidationAmounts}
 */
module.exports.calculate_liquidation_amounts_js = function (inputs) {
  const ret = wasm.calculate_liquidation_amounts_js(addHeapObject(inputs))
  return takeObject(ret)
}

function handleError(f, args) {
  try {
    return f.apply(this, args)
  } catch (e) {
    wasm.__wbindgen_exn_store(addHeapObject(e))
  }
}

module.exports.__wbindgen_object_clone_ref = function (arg0) {
  const ret = getObject(arg0)
  return addHeapObject(ret)
}

module.exports.__wbindgen_is_undefined = function (arg0) {
  const ret = getObject(arg0) === undefined
  return ret
}

module.exports.__wbindgen_object_drop_ref = function (arg0) {
  takeObject(arg0)
}

module.exports.__wbindgen_string_get = function (arg0, arg1) {
  const obj = getObject(arg1)
  const ret = typeof obj === 'string' ? obj : undefined
  var ptr1 = isLikeNone(ret)
    ? 0
    : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc)
  var len1 = WASM_VECTOR_LEN
  getInt32Memory0()[arg0 / 4 + 1] = len1
  getInt32Memory0()[arg0 / 4 + 0] = ptr1
}

module.exports.__wbg_parse_670c19d4e984792e = function () {
  return handleError(function (arg0, arg1) {
    const ret = JSON.parse(getStringFromWasm0(arg0, arg1))
    return addHeapObject(ret)
  }, arguments)
}

module.exports.__wbg_stringify_e25465938f3f611f = function () {
  return handleError(function (arg0) {
    const ret = JSON.stringify(getObject(arg0))
    return addHeapObject(ret)
  }, arguments)
}

module.exports.__wbindgen_throw = function (arg0, arg1) {
  throw new Error(getStringFromWasm0(arg0, arg1))
}

const path = require('path').join(__dirname, 'index_bg.wasm')
const bytes = require('fs').readFileSync(path)

const wasmModule = new WebAssembly.Module(bytes)
const wasmInstance = new WebAssembly.Instance(wasmModule, imports)
wasm = wasmInstance.exports
module.exports.__wasm = wasm
