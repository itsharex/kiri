/** Serial field-level updates shared by independent annotation windows. */
export class AppearanceUpdates {
  constructor(initial) {
    this.current = initial;
    this.saved = initial;
    this.pending = {};
    this.inFlight = null;
    this.events = 0;
    this.saveEvents = 0;
  }
  update(next, renderBaseline = this.current) {
    for (const key of Object.keys(next)) {
      // The event handler can still hold the previous render's full snapshot
      // after receive() has published another window's changes.
      if (next[key] !== renderBaseline[key]) this.pending[key] = next[key];
    }
    this.publish();
  }
  receive(saved) {
    this.saved = saved;
    this.events += 1;
    this.publish();
  }
  publish() { this.current = { ...this.saved, ...this.inFlight, ...this.pending }; }
  get hasPending() { return Object.keys(this.pending).length > 0; }
  beginSave() {
    if (this.inFlight || !this.hasPending) return null;
    this.inFlight = this.pending;
    this.pending = {};
    this.saveEvents = this.events;
    return this.inFlight;
  }
  finishSave(saved) {
    // A later event can already describe a newer cross-window write.
    if (this.events === this.saveEvents) this.saved = saved;
    this.inFlight = null;
    this.publish();
  }
  failSave() {
    this.pending = { ...this.inFlight, ...this.pending };
    this.inFlight = null;
    this.publish();
  }
}
