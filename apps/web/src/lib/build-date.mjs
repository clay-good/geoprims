// A tagged build supplies the commit timestamp so rebuilding it later keeps
// date-dependent page copy and sitemap dates byte-identical.
export function buildDate(epoch = process.env.SOURCE_DATE_EPOCH) {
  if (epoch === undefined) return new Date().toISOString().slice(0, 10);
  if (!/^\d+$/.test(epoch)) throw new Error('SOURCE_DATE_EPOCH must be Unix seconds');
  const date = new Date(Number(epoch) * 1000);
  if (!Number.isFinite(date.valueOf())) throw new Error('SOURCE_DATE_EPOCH is outside the supported date range');
  return date.toISOString().slice(0, 10);
}
