export type KiriResourceRoute =
  | "capture"
  | "thumbnail"
  | "annotation-source"
  | "asset"
  | "media";

export function kiriResourceUrl(
  route: KiriResourceRoute,
  segments?: readonly string[],
  query?: Readonly<Record<string, string | number>>,
): string;

export function configureVideoPlaybackOrigin(origin?: string | null): void;
export function videoResourceUrl(id: string): string;
export function videoResourceCrossOrigin(src: string): "anonymous" | undefined;
