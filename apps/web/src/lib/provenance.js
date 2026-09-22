// The provenance of the answer on screen (web/app-shell, "Result panel": the
// `meta` in an expandable section). Where the method is described once in
// "How we got this", this is the record for this very result: which tool and
// core versions computed it, which reference data it read, and what it cites.

/** Rows for a result's meta: [{ label, value }]. `registry` names the datasets. */
export function provenanceRows(meta, registry) {
  if (!meta) return [];
  const dataset = (a) => {
    const r = (registry?.assets ?? []).find((x) => x.id === a.id);
    return r ? `${r.title}, version ${a.version}` : `version ${a.version} of a reference dataset`;
  };
  const rows = [
    { label: 'Computed by', value: `${meta.tool} ${meta.toolVersion}, core ${meta.coreVersion}` },
    meta.model && { label: 'Model', value: meta.model },
    meta.accuracy && { label: 'Accuracy', value: meta.accuracy },
    (meta.assets ?? []).length > 0 && { label: 'Reference data', value: meta.assets.map(dataset).join('; ') },
    { label: 'Notes', value: (meta.warnings ?? []).length ? `${meta.warnings.length} shown with the answer` : 'None' },
    (meta.references ?? []).length > 0 && { label: 'Cites', value: meta.references.map((r) => [r.issuer, r.title].filter(Boolean).join(', ')).join('; ') },
  ];
  return rows.filter(Boolean);
}
