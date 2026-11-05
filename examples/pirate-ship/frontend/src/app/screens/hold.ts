import { KeyValuePipe } from '@angular/common';
import { Component, OnInit, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ApiService, Hold } from '../api.service';

@Component({
  selector: 'app-hold',
  imports: [FormsModule, KeyValuePipe],
  template: `
    <h1>Below Deck</h1>
    @if (hold(); as h) {
      <div class="tiles">
        <section class="card">
          <h2>The barrel</h2>
          <div
            class="barrel"
            role="meter"
            aria-label="Hold weight"
            [attr.aria-valuenow]="h.weight_stone"
            aria-valuemin="0"
            [attr.aria-valuemax]="h.capacity_stone"
          >
            <div class="barrel-fill" [style.height.%]="percent()"></div>
          </div>
          <p class="figure">{{ h.weight_stone }} of {{ h.capacity_stone }} stone</p>
        </section>
        <section class="card">
          <h2>What she carries</h2>
          <table>
            <thead><tr><th scope="col">Item</th><th scope="col" class="num">Count</th></tr></thead>
            <tbody>
              @for (c of h.counts | keyvalue; track c.key) {
                <tr><td>{{ c.key }}</td><td class="num">{{ c.value }}</td></tr>
              }
            </tbody>
          </table>
        </section>
        @if (api.can('change_hold')) {
          <section class="card">
            <h2>Load or unload</h2>
            <form (ngSubmit)="change()">
              <label>
                Item
                <select name="item" [(ngModel)]="item">
                  @for (c of h.counts | keyvalue; track c.key) { <option [value]="c.key">{{ c.key }}</option> }
                </select>
              </label>
              <label>Amount (negative to unload) <input name="amount" type="number" step="1" [(ngModel)]="amount" required /></label>
              <label>Reason <input name="reason" [(ngModel)]="reason" maxlength="120" required /></label>
              <button type="submit">Log it</button>
            </form>
          </section>
        }
      </div>
    }
  `,
})
export class HoldScreen implements OnInit {
  protected api = inject(ApiService);
  protected hold = signal<Hold | null>(null);
  protected percent = computed(() => {
    const h = this.hold();
    return h ? (100 * h.weight_stone) / h.capacity_stone : 0;
  });
  item = 'rum';
  amount = 1;
  reason = '';

  async ngOnInit() {
    this.hold.set(await this.api.hold());
  }

  async change() {
    const answer = await this.api.changeHold(this.item, this.amount, this.reason);
    if (answer) {
      this.hold.set(answer);
      this.reason = '';
    }
  }
}
