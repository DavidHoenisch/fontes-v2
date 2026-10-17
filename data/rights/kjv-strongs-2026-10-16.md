# Approved KJV dictionary annotations

Owner decision, 2026-10-16: include GPL KJV annotation data with license and source files. This is a specific exception to the initial public-domain source policy, separate from the approved CC BY Hebrew source.

Exact input: eBible `eng-kjv_usfm.zip?x=1`, 2,766,294 bytes, SHA-256 `05b021ab465bd283e7df02409c78436487d582276b3a0b60d6fcf5d967a0af14`. This source is retained unchanged alongside each Study build, in editable USFM form. Conversion code: `crates/fontes-data-pipeline/src/word_tags.rs`; reviewed mapping: `data/mappings/kjv-strongs-v1.json`.

Annotation provenance: Bible Foundation (OT Strong’s tagging); KJV2003/CrossWire and Dr. Maurice A. Robinson (NT Strong’s tagging); Michael Paul Johnson/eBible.org (formatting). Copyright © 2003–2023 CrossWire Bible Society for its markup and tagging contributions. Preserve underlying KJV Crown/regional rights separately.

Upstream declares “Distribution License GPL” without a version:
- https://crosswire.org/sword/modules/ModInfo.jsp?modName=KJV
- https://wiki.crosswire.org/CrossWire_KJV

The annotation derivative is distributed under GNU GPL version 3, included as `GPL-3.0.txt`. GPL section 14 permits choosing a published version where no version is specified. This notice does not relabel the annotations public domain or CC BY, and does not change the separate licensing of the application or other sources.

Changes: extract supplied Strong’s dictionary associations into exact edition-bound Unicode-scalar spans; normalize whitespace and paragraph marks only for comparison; retain the installed reading text; withhold all tags for 1 John 2:23 because source `[but]` differs from installed `but`. No Hebrew/Greek token identity or English-to-original-token alignment is inferred. The build retains normalized audit JSONL and the exact source archive. No warranty; see the accompanying GPL text.
