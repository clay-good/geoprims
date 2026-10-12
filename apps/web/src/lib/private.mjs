/** Returns the inputs that the manifest allows the app to persist or share. */
export function publicInputs(tool, inputs) {
  const properties = tool.inputs?.properties ?? {};
  return Object.fromEntries(
    Object.entries(inputs).filter(([name]) => properties[name]?.['x-private'] !== true),
  );
}
