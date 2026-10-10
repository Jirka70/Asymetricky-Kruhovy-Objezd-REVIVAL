type AdmissionOffer = { prihlasky: number; prijati?: number | null };

export function admissionStats(offers: readonly AdmissionOffer[]) {
  if (offers.length === 0) return null;
  const applications = offers.reduce((sum, offer) => sum + offer.prihlasky, 0);
  // All specializations must have a count before presenting the total or ratio.
  const accepted = offers.every((offer) => offer.prijati != null)
    ? offers.reduce((sum, offer) => sum + offer.prijati!, 0)
    : null;
  return {
    applications,
    accepted,
    rate: accepted !== null && applications > 0 ? accepted / applications : null,
  };
}
