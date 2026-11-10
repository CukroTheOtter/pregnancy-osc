// Privacy mode turned on by itself while a streaming or recording app runs
// (Settings > Look). Never saved: it goes off again when that app closes.

export const autoPrivacy = $state({
  /** The recording app that's running (when the setting is on). */
  app: null as string | null,
  /** Turned off with the 🙈 pill: stays off until that app closes. */
  off: false,
});

/** Why privacy mode is on by itself (the app's name), or null. */
export function autoPrivacyReason(): string | null {
  return autoPrivacy.app && !autoPrivacy.off ? autoPrivacy.app : null;
}

/** From the status: the recording app found, if the setting is on. */
export function syncAutoPrivacy(recordingApp: string | null | undefined, enabled: boolean) {
  const found = enabled ? (recordingApp ?? null) : null;
  if (found !== autoPrivacy.app) {
    autoPrivacy.app = found;
    // A new app (or none): the pill's "off" was for the last one.
    autoPrivacy.off = false;
  }
}
