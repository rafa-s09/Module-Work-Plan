import { Component, OnInit, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { ApiService, Chest, Hold, Voyage } from '../api.service';
import { SeaChart } from './sea-chart';

@Component({
  selector: 'app-chart',
  imports: [RouterLink, SeaChart],
  template: `
    <h1>The Chart</h1>
    @if (voyage(); as v) {
      <section class="card">
        <h2>{{ v.name }}, bound for {{ v.destination }}</h2>
        <app-sea-chart [state]="v.state" [lastStop]="lastStop()" />
        <p><a [routerLink]="['/voyages', v.id, 'plunder']">See the plunder</a></p>
      </section>
    } @else {
      <section class="card">
        <h2>No voyage charted</h2>
        <app-sea-chart state="docked" />
        <p><a routerLink="/voyages">Chart a course</a></p>
      </section>
    }
    <div class="tiles">
      <section class="card">
        <h2>The Hold</h2>
        @if (hold(); as h) {
          <p class="figure">{{ h.weight_stone }} of {{ h.capacity_stone }} stone</p>
        }
      </section>
      <section class="card">
        <h2>The Chest</h2>
        <p class="figure">{{ chest()?.balance_display }}</p>
      </section>
    </div>
  `,
})
export class ChartScreen implements OnInit {
  private api = inject(ApiService);
  private voyages = signal<Voyage[]>([]);
  protected hold = signal<Hold | null>(null);
  protected chest = signal<Chest | null>(null);

  protected voyage = computed(
    () => this.voyages().find((v) => ['sailing', 'plundering', 'returning'].includes(v.state)) ?? this.voyages()[0],
  );
  protected lastStop = computed(() => {
    const events = this.voyage()?.events ?? [];
    return events.at(-2)?.state ?? 'sailing';
  });

  async ngOnInit() {
    this.voyages.set((await this.api.voyages()) ?? []);
    this.hold.set(await this.api.hold());
    this.chest.set(await this.api.chest());
  }
}
