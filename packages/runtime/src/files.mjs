// Private local-file boundary shared by browser and Node hosts. This check runs
// before bytes are copied into a worker or Wasm memory.
export const MAX_WHOLE_FILE_BYTES = 64 * 1024 * 1024;

export function checkFileSize(size) {
  if (!Number.isSafeInteger(size) || size < 0) throw new TypeError('A local file size must be a nonnegative integer.');
  if (size > MAX_WHOLE_FILE_BYTES) {
    throw new RangeError(`A whole file must be at most ${MAX_WHOLE_FILE_BYTES} bytes.`);
  }
}

export function fileBytes(value) {
  let bytes;
  if (value instanceof Uint8Array) bytes = value;
  else if (value instanceof ArrayBuffer) bytes = new Uint8Array(value);
  else throw new TypeError('Local file bytes must be a Uint8Array or ArrayBuffer.');
  checkFileSize(bytes.length);
  return bytes;
}
