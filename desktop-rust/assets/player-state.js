// Shared small decisions; no network or persistence. Also exercised in Node.
(function (root) {
    const label = value => String(value || '').trim().toLowerCase();
    root.PiratePlayerState = {
        canStart: (nativeVisible, documentHidden) => nativeVisible !== false && !documentHidden,
        startupReady(duration, position, buffered) {
            const remaining = Number.isFinite(duration) && duration > 0 ? Math.max(0, duration-position) : 18;
            return remaining > 0 && buffered + 0.25 >= Math.min(18,remaining);
        },
        preferredTrack(tracks, preference) {
            if (!preference) return -1;
            const exact = tracks.findIndex(track => label(track.Language) === label(preference.language)
                && label(track.Title) === label(preference.title));
            if (exact >= 0) return exact;
            return preference.language ? tracks.findIndex(track => label(track.Language) === label(preference.language)) : -1;
        },
        shouldPrepare: (remaining, buffered, visible, prepared) => visible && !prepared
            && Number.isFinite(remaining) && remaining > 0 && remaining <= 120 && buffered >= 30,
        subtitleVtt(text) {
            const normalized = text.replace(/^\uFEFF/, '').replace(/\r\n?/g, '\n');
            return /^WEBVTT\b/.test(normalized) ? normalized
                : 'WEBVTT\n\n' + normalized.replace(/(\d{2}:\d{2}:\d{2}),(\d{3})/g, '$1.$2');
        },
    };
    if (typeof module !== 'undefined') module.exports = root.PiratePlayerState;
})(typeof window === 'undefined' ? globalThis : window);
