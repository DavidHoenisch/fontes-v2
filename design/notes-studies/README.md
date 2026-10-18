# Notes interaction study

Reviewable HTML prototype only; no installed-app changes. Uses sample observations, references and KJV excerpts, never real user notes. Open through the repository preview server at /design/notes-studies/index.html.

Proposed: searchable note previews with reference context, Notes/Highlights/Bookmarks filters, recent-first sorting, a pinned note, bottom New note action, focused editing with passage context, deliberate Save/unsaved-change protection, empty and no-results states. Palette and state controls live outside the phone preview. In-memory changes reset on reload; the “Private on this device” wording illustrates the production storage model, not prototype persistence.

Search, filters, sorting, note editing/creation, passage dialog and empty state are interactive. Other app destinations show a preview notice. Pinning, sorting metadata and cross-annotation search are design proposals requiring engine/storage assessment before production. No claim that those features are already implemented. Existing native delete confirmation should be retained when implementing note management.

Reuses bundled IBM Plex Sans, Cormorant Garamond Italic (SIL OFL, assets/ui/licenses) and original inline outline icons. Standalone CSS preserves local/offline preview behavior without adding a build chain. Motion respects reduced-motion preferences. Native integration is separate work after review.
