import type { AppearanceSettings } from "./model";
export class AppearanceUpdates {
  constructor(initial: AppearanceSettings);
  current: AppearanceSettings;
  saved: AppearanceSettings;
  pending: Partial<AppearanceSettings>;
  inFlight: Partial<AppearanceSettings> | null;
  events: number;
  hasPending: boolean;
  update(next: AppearanceSettings, renderBaseline?: AppearanceSettings): void;
  receive(saved: AppearanceSettings): void;
  beginSave(): Partial<AppearanceSettings> | null;
  finishSave(saved: AppearanceSettings): void;
  failSave(): void;
}
