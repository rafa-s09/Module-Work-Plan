import { Component, OnInit, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ApiService, Member } from '../api.service';

// Choices for the selects only. The back decides what each one means and may refuse any of them.
const RANKS = ['captain', 'quartermaster', 'boatswain', 'gunner', 'sailor', 'cabin_boy'];
const STATUSES = ['aboard', 'ashore', 'marooned', 'dead'];

@Component({
  selector: 'app-crew',
  imports: [FormsModule],
  template: `
    <h1>Ship's Articles</h1>
    <section class="card">
      <table class="stack">
        <caption>Every soul who ever signed</caption>
        <thead>
          <tr><th scope="col">Name</th><th scope="col">Rank</th><th scope="col">Shares</th><th scope="col">Status</th></tr>
        </thead>
        <tbody>
          @for (m of crew(); track m.id) {
            <tr>
              <td data-label="Name">{{ m.name }}</td>
              <td data-label="Rank">
                @if (api.can('manage_crew')) {
                  <select [attr.aria-label]="'Rank of ' + m.name" (change)="change(m.id, 'rank', $event)">
                    @for (r of ranks; track r) { <option [value]="r" [selected]="r === m.rank">{{ r }}</option> }
                  </select>
                } @else { {{ m.rank }} }
              </td>
              <td data-label="Shares">{{ m.shares_display }}</td>
              <td data-label="Status">
                @if (api.can('manage_crew')) {
                  <select [attr.aria-label]="'Status of ' + m.name" (change)="change(m.id, 'status', $event)">
                    @for (s of statuses; track s) { <option [value]="s" [selected]="s === m.status">{{ s }}</option> }
                  </select>
                } @else { {{ m.status }} }
              </td>
            </tr>
          }
        </tbody>
      </table>
    </section>

    <div class="tiles">
      @if (api.can('manage_crew')) {
        <section class="card">
          <h2>Sign a new hand</h2>
          <form (ngSubmit)="add()">
            <label>Name <input name="name" [(ngModel)]="name" maxlength="60" required /></label>
            <label>
              Rank
              <select name="rank" [(ngModel)]="rank">
                @for (r of ranks; track r) { <option [value]="r">{{ r }}</option> }
              </select>
            </label>
            <button type="submit">Sign the articles</button>
          </form>
        </section>
      }
      @if (api.can('manage_accounts')) {
        <section class="card">
          <h2>Accounts</h2>
          <form (ngSubmit)="account()">
            <label>
              Crew member
              <select name="member" [(ngModel)]="accountMember" required>
                @for (m of crew(); track m.id) { <option [ngValue]="m.id">{{ m.name }}</option> }
              </select>
            </label>
            <label>Username (only for a new account) <input name="username" [(ngModel)]="username" autocomplete="off" /></label>
            <label>Password <input name="password" type="password" [(ngModel)]="password" autocomplete="new-password" required /></label>
            <div class="actions">
              <button type="submit">Give an account</button>
              <button type="button" class="ghost" (click)="reset()">Reset the password</button>
            </div>
          </form>
        </section>
      }
      <section class="card">
        <h2>Yer own password</h2>
        <form (ngSubmit)="changeOwn()">
          <label>Current <input name="current" type="password" [(ngModel)]="current" autocomplete="current-password" required /></label>
          <label>New <input name="next" type="password" [(ngModel)]="next" autocomplete="new-password" required /></label>
          <button type="submit">Change it</button>
        </form>
        @if (done()) { <p role="status">{{ done() }}</p> }
      </section>
    </div>
  `,
})
export class CrewScreen implements OnInit {
  protected api = inject(ApiService);
  protected crew = signal<Member[]>([]);
  protected done = signal('');
  protected ranks = RANKS;
  protected statuses = STATUSES;
  name = '';
  rank = 'sailor';
  accountMember: number | null = null;
  username = '';
  password = '';
  current = '';
  next = '';

  async ngOnInit() {
    await this.reload();
  }

  private async reload() {
    this.crew.set((await this.api.crew()) ?? []);
  }

  async change(id: number, field: 'rank' | 'status', event: Event) {
    const select = event.target as HTMLSelectElement;
    await this.api.changeMember(id, { [field]: select.value });
    // Reloaded on a refusal too, so the select goes back to what the back says.
    const bottle = this.api.bottle();
    await this.reload();
    this.api.bottle.set(bottle);
    select.value = this.crew().find((m) => m.id === id)?.[field] ?? select.value;
  }

  async add() {
    if (await this.api.addMember(this.name, this.rank)) {
      this.name = '';
      await this.reload();
    }
  }

  async account() {
    if (this.accountMember === null) return;
    if (await this.api.createAccount(this.accountMember, this.username, this.password)) {
      this.done.set(`Account ${this.username} is ready.`);
      this.username = this.password = '';
    }
  }

  async reset() {
    if (this.accountMember === null) return;
    if (await this.api.resetPassword(this.accountMember, this.password)) {
      this.done.set('Password reset.');
      this.password = '';
    }
  }

  async changeOwn() {
    if (await this.api.changeOwnPassword(this.current, this.next)) {
      this.done.set('Password changed.');
      this.current = this.next = '';
    }
  }
}
