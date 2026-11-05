import { Component, OnInit, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { ApiService, Voyage } from '../api.service';

const ACTIONS: Record<string, string> = {
  sailing: 'Hoist the sails',
  plundering: 'Attack!',
  returning: 'Head for home',
  docked: 'Drop anchor',
  sunk: 'She sinks!',
};

@Component({
  selector: 'app-voyages',
  imports: [FormsModule, RouterLink],
  template: `
    <h1>The Course</h1>
    @if (api.can('move_ship')) {
      <section class="card">
        <h2>Chart a new voyage</h2>
        <form class="inline" (ngSubmit)="create()">
          <label>Name <input name="name" [(ngModel)]="name" maxlength="80" required /></label>
          <label>Destination <input name="destination" [(ngModel)]="destination" maxlength="80" required /></label>
          <button type="submit">Chart it</button>
        </form>
      </section>
    }
    @for (v of voyages(); track v.id) {
      <section class="card">
        <h2>{{ v.name }}, bound for {{ v.destination }}</h2>
        <p><span class="state">{{ v.state }}</span> · {{ v.roster.length }} on the roster</p>
        <div class="actions">
          <a [routerLink]="['/voyages', v.id, 'plunder']">Divide the plunder</a>
          @if (api.can('move_ship')) {
            @for (next of v.next_states; track next) {
              <button type="button" [class.danger]="next === 'sunk'" (click)="move(v.id, next)">{{ actions[next] }}</button>
            }
          }
        </div>
      </section>
    } @empty {
      <p>No voyages yet. The sea is waiting.</p>
    }
  `,
})
export class VoyagesScreen implements OnInit {
  protected api = inject(ApiService);
  protected voyages = signal<Voyage[]>([]);
  protected actions = ACTIONS;
  name = '';
  destination = '';

  async ngOnInit() {
    await this.reload();
  }

  private async reload() {
    this.voyages.set((await this.api.voyages()) ?? []);
  }

  async create() {
    if (await this.api.createVoyage(this.name, this.destination)) {
      this.name = this.destination = '';
      await this.reload();
    }
  }

  async move(id: number, state: string) {
    if (await this.api.changeVoyageState(id, state)) await this.reload();
  }
}
