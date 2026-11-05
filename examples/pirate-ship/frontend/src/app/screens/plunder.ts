import { Component, OnInit, inject, input, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ApiService, Plunder, Split, Voyage } from '../api.service';

// Choices for the select only. The amounts are the back's (loot#FR-03).
const INJURIES = ['right_arm', 'left_arm', 'right_leg', 'left_leg', 'eye', 'finger'];

@Component({
  selector: 'app-plunder',
  imports: [FormsModule],
  template: `
    <h1>Divide the Plunder</h1>
    @if (voyage(); as v) {
      <p><strong>{{ v.name }}</strong> · <span class="state">{{ v.state }}</span></p>

      <div class="tiles">
        <section class="card">
          <h2>The plunder</h2>
          <ul>
            @for (p of plunder()?.plunder; track $index) {
              <li>{{ p.description }}: <span class="money">{{ p.amount_display }}</span></li>
            } @empty { <li>Not a single real yet.</li> }
          </ul>
          @if (api.can('manage_loot') && !split() && v.state === 'plundering') {
            <form (ngSubmit)="addPlunder()">
              <label>Reales <input name="reales" type="number" min="1" step="1" [(ngModel)]="reales" required /></label>
              <label>What was it <input name="description" [(ngModel)]="description" maxlength="120" required /></label>
              <button type="submit">Record it</button>
            </form>
          }
        </section>

        <section class="card">
          <h2>The wounded</h2>
          <ul>
            @for (c of plunder()?.compensations; track $index) {
              <li>{{ c.name }}, {{ c.injury }}: <span class="money">{{ c.amount_display }}</span></li>
            } @empty { <li>All limbs accounted for.</li> }
          </ul>
          @if (api.can('manage_loot') && !split() && v.roster.length) {
            <form (ngSubmit)="addCompensation()">
              <label>
                Who
                <select name="member" [(ngModel)]="member" required>
                  @for (r of v.roster; track r.crew_member_id) { <option [ngValue]="r.crew_member_id">{{ r.name }}</option> }
                </select>
              </label>
              <label>
                Injury
                <select name="injury" [(ngModel)]="injury">
                  @for (i of injuries; track i) { <option [value]="i">{{ i }}</option> }
                </select>
              </label>
              <button type="submit">Compensate</button>
            </form>
          }
        </section>
      </div>

      <section class="card">
        <h2>The split</h2>
        @if (split(); as s) {
          <p>
            Plunder <span class="money">{{ s.plunder_display }}</span> ·
            compensation <span class="money">{{ s.compensation_display }}</span> ·
            left over for the chest <span class="money">{{ s.leftover_display }}</span>
          </p>
          <table class="stack">
            <thead>
              <tr>
                <th scope="col">Name</th><th scope="col">Shares</th><th scope="col">Compensation</th>
                <th scope="col">Share</th><th scope="col">Total</th><th scope="col">Paid to</th>
              </tr>
            </thead>
            <tbody>
              @for (l of s.lines; track l.crew_member_id) {
                <tr>
                  <td data-label="Name">{{ l.name }}</td>
                  <td data-label="Shares">{{ l.shares_display }}</td>
                  <td data-label="Compensation" class="money">{{ l.compensation_display }}</td>
                  <td data-label="Share" class="money">{{ l.share_display }}</td>
                  <td data-label="Total" class="money">{{ l.total_display }}</td>
                  <td data-label="Paid to">{{ l.paid_to === 'chest' ? 'the chest' : 'the member' }}</td>
                </tr>
              }
            </tbody>
          </table>
        } @else {
          <p>Not divided yet.</p>
          @if (api.can('manage_loot')) {
            <button type="button" (click)="runSplit()">Divide it, once and for all</button>
          }
        }
      </section>
    }
  `,
})
export class PlunderScreen implements OnInit {
  protected api = inject(ApiService);
  /** The route's :id. */
  readonly id = input.required<string>();

  protected voyage = signal<Voyage | null>(null);
  protected plunder = signal<Plunder | null>(null);
  protected split = signal<Split | null>(null);
  protected injuries = INJURIES;
  reales: number | null = null;
  description = '';
  member: number | null = null;
  injury = 'finger';

  private get voyageId() {
    return Number(this.id());
  }

  async ngOnInit() {
    this.voyage.set(await this.api.voyage(this.voyageId));
    this.plunder.set(await this.api.plunder(this.voyageId));
    this.split.set(await this.api.split(this.voyageId));
  }

  async addPlunder() {
    const answer = await this.api.addPlunder(this.voyageId, this.reales ?? 0, this.description);
    if (answer) {
      this.plunder.set(answer);
      this.reales = null;
      this.description = '';
    }
  }

  async addCompensation() {
    if (this.member === null) return;
    const answer = await this.api.addCompensation(this.voyageId, this.member, this.injury);
    if (answer) this.plunder.set(answer);
  }

  async runSplit() {
    const answer = await this.api.runSplit(this.voyageId);
    if (answer) this.split.set(answer);
  }
}
