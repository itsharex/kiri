/** Missing or unrecognized capabilities never opt a renderer into an operation. */
export function videoEditingCapabilities(capabilities) {
  const videoEditing = capabilities?.videoEditing === true;
  return {
    videoEditing,
    videoSpeedEditing: videoEditing && capabilities?.videoSpeedEditing === true,
    videoEffectsEditing: videoEditing && capabilities?.videoEffectsEditing === true,
    videoAnnotationsEditing: videoEditing && capabilities?.videoAnnotationsEditing === true,
    videoExportPresets: videoEditing && capabilities?.videoExportPresets === true,
  };
}

/** Inspect without rewriting the draft: unsupported content must never be dropped. */
export function unsupportedVideoProjectFeatures(project, capabilities) {
  const caps = videoEditingCapabilities(capabilities);
  if (!caps.videoEditing) return ["editing"];
  const unsupported = [];
  if (!caps.videoSpeedEditing && project.edit.segments.some(segment => (segment.speed ?? 1) !== 1)) unsupported.push("speed");
  if (!caps.videoEffectsEditing && project.edit.effects.length) unsupported.push("effects");
  if (!caps.videoAnnotationsEditing && project.edit.annotations.length) unsupported.push("annotations");
  if (!caps.videoAnnotationsEditing && project.edit.stickers.length) unsupported.push("stickers");
  if (!caps.videoExportPresets && project.preset !== "original") unsupported.push("presets");
  return unsupported;
}

export function isBasicVideoEditing(capabilities) {
  const caps = videoEditingCapabilities(capabilities);
  return caps.videoEditing && !caps.videoSpeedEditing && !caps.videoEffectsEditing && !caps.videoAnnotationsEditing;
}
