export function captureForeignConfigDisclosureState(
  disclosures: Iterable<{ open: boolean; dataset: { disclosureKey?: string } }>,
) {
  return new Set(
    Array.from(disclosures)
      .filter((details) => details.open)
      .map((details) => details.dataset.disclosureKey)
      .filter((key): key is string => Boolean(key)),
  );
}

export function formatForeignConfigProbeSummary(successes: number, attempts: number, averageMs: number | null | undefined) {
  return `${successes}/${attempts} успешных${averageMs == null ? '' : ` · ${averageMs} мс`}`;
}
