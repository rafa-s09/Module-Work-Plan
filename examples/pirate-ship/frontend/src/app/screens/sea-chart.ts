import { Component, computed, input } from '@angular/core';

const STOPS = [
  { state: 'docked', label: 'In port', x: 90, y: 250 },
  { state: 'sailing', label: 'Under sail', x: 270, y: 120 },
  { state: 'plundering', label: 'Plundering', x: 480, y: 210 },
  { state: 'returning', label: 'Homeward', x: 660, y: 90 },
];
const RHUMB_ANGLES = Array.from({ length: 16 }, (_, i) => (i * Math.PI) / 8);

/** The voyage as stops on a dashed route. Drawing only: the state comes from the back. */
@Component({
  selector: 'app-sea-chart',
  template: `
    <svg viewBox="0 0 760 320" role="img" [attr.aria-label]="description()" class="sea-chart">
      <rect width="760" height="320" class="water" />
      @for (line of rhumbs; track line.x) {
        <line x1="380" y1="160" [attr.x2]="line.x" [attr.y2]="line.y" class="rhumb" />
      }
      <path class="land" d="M0 270 Q40 225 95 262 T170 320 H0Z" />
      <path class="land" d="M430 262 Q480 232 540 262 T590 296 Q510 318 442 296Z" />
      <path class="land" d="M610 26 Q660 4 716 30 T748 72 Q680 62 628 56Z" />
      <path [attr.d]="route" class="route" />
      @for (stop of stops; track stop.state) {
        <circle [attr.cx]="stop.x" [attr.cy]="stop.y" r="7" class="stop" [class.here]="stop.state === state()" />
        <text [attr.x]="stop.x" [attr.y]="stop.y + 28" text-anchor="middle" class="stop-label">{{ stop.label }}</text>
      }
      <image href="theme/compass-rose.svg" x="340" y="10" width="80" height="80" />
      @if (where(); as at) {
        <image
          href="theme/ship.svg"
          [attr.x]="at.x - 32"
          [attr.y]="at.y - 66"
          width="64"
          height="64"
          [class.sunk]="state() === 'sunk'"
        />
      }
      @if (state() === 'sunk') {
        <text x="380" y="300" text-anchor="middle" class="stop-label">Gone to Davy Jones</text>
      }
    </svg>
  `,
})
export class SeaChart {
  state = input.required<string>();
  /** Where she went down; a sunk voyage no longer has a stop of its own. */
  lastStop = input('sailing');

  protected stops = STOPS;
  protected rhumbs = RHUMB_ANGLES.map((a) => ({
    x: Math.round(380 + 800 * Math.cos(a)),
    y: Math.round(160 + 800 * Math.sin(a)),
  }));
  protected route = 'M' + STOPS.map((s) => `${s.x} ${s.y}`).join(' L');

  protected where = computed(() => {
    const state = this.state() === 'sunk' ? this.lastStop() : this.state();
    return STOPS.find((s) => s.state === state);
  });
  protected description = computed(() => `Sea chart. The ship is ${this.state()}.`);
}
