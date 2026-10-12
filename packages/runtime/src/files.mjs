// Private local-file boundary shared by browser and Node hosts. This check runs
// before bytes are copied into a worker or Wasm memory.
export const MAX_WHOLE_FILE_BYTES = 64 * 1024 * 1024;
export const MAX_SPARSE_FILE_BYTES = 16 * 1024 * 1024 * 1024;

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

export function rangeSource(value) {
  if (!value || !Number.isSafeInteger(value.size) || value.size <= 0 || value.size > MAX_SPARSE_FILE_BYTES) {
    throw new RangeError(`A range-backed file size must be from 1 to ${MAX_SPARSE_FILE_BYTES} bytes.`);
  }
  if (typeof value.slice !== 'function') throw new TypeError('A range-backed file must provide slice(start, end).');
  return value;
}

export async function sourceRange(source, offset, length) {
  if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(length) || offset < 0 || length <= 0 || offset + length > source.size) {
    throw new RangeError('The requested local-file range is outside the declared file.');
  }
  let value = source.slice(offset, offset + length);
  if (value && typeof value.arrayBuffer === 'function') value = await value.arrayBuffer();
  else value = await value;
  let bytes;
  if (value instanceof Uint8Array) bytes = value;
  else if (value instanceof ArrayBuffer) bytes = new Uint8Array(value);
  else throw new TypeError('A local-file slice must return bytes, an ArrayBuffer, or a Blob.');
  if (bytes.length !== length) throw new RangeError(`The local-file slice returned ${bytes.length} bytes; ${length} were requested.`);
  return bytes;
}
