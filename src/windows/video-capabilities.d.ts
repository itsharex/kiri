import type { PlatformCapabilitiesDto } from "../lib/ipc";
import type { VideoProject } from "./video-project";

export type VideoEditingCapabilities = Pick<PlatformCapabilitiesDto,
  "videoEditing" | "videoSpeedEditing" | "videoEffectsEditing" | "videoAnnotationsEditing" | "videoExportPresets">;
export type UnsupportedVideoProjectFeature = "editing" | "speed" | "effects" | "annotations" | "stickers" | "presets";
export function videoEditingCapabilities(capabilities?: Partial<VideoEditingCapabilities> | null): VideoEditingCapabilities;
export function unsupportedVideoProjectFeatures(project: Pick<VideoProject, "edit" | "preset">, capabilities?: Partial<VideoEditingCapabilities> | null): UnsupportedVideoProjectFeature[];
export function isBasicVideoEditing(capabilities?: Partial<VideoEditingCapabilities> | null): boolean;
