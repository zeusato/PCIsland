/** Match the exact track first, then a regional variant of the preferred language. */
export function preferredTrack<T>(tracks: T[], key: (track: T) => string, preferred: string): T | undefined {
  const wanted = preferred.toLowerCase();
  const exact = tracks.find((track) => key(track).toLowerCase() === wanted);
  return exact ?? tracks.find((track) => key(track).toLowerCase().split('-')[0] === wanted.split('-')[0]);
}

type ManifestTrack = { lang?: string; name: string };

export function manifestLanguage(track: ManifestTrack): string {
  if (track.lang && track.lang !== 'und') return track.lang;
  if (/vietnamese|tiếng việt/i.test(track.name)) return 'vi';
  return track.name;
}

export function languageLabel(language: string, locale: string, unnamed: string): string {
  if (!language) return unnamed;
  try {
    return new Intl.DisplayNames(locale, { type: 'language' }).of(language) ?? language;
  } catch {
    return language;
  }
}
