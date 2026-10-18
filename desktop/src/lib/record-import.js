export function acceptedRecordSelection(preview, selectedIndices) {
  return preview.entries.flatMap((entry, index) =>
    entry.verdict === "accept" && (!selectedIndices || selectedIndices.includes(index)) ? [{ index }] : [],
  );
}