import { Component, inject } from '@angular/core';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { ApiService } from './api.service';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  template: `
    @if (api.loggedIn()) {
      <header class="topbar">
        <a class="brand" routerLink="/">
          <img src="theme/compass-rose.svg" alt="" width="36" height="36" />
          The Black Ledger
        </a>
        <nav aria-label="Main">
          <a routerLink="/" routerLinkActive="active" [routerLinkActiveOptions]="{ exact: true }">The Chart</a>
          <a routerLink="/crew" routerLinkActive="active">Ship's Articles</a>
          <a routerLink="/voyages" routerLinkActive="active">The Course</a>
          <a routerLink="/hold" routerLinkActive="active">Below Deck</a>
        </nav>
        <div class="who">
          <span>{{ api.me()?.name }}</span>
          <button type="button" class="ghost" (click)="api.logout()">Go ashore</button>
        </div>
      </header>
    }
    @if (api.bottle()) {
      <p class="bottle" role="alert">
        <span aria-hidden="true">🍾</span> {{ api.bottle() }}
        <button type="button" class="ghost" (click)="api.bottle.set('')">Toss it back</button>
      </p>
    }
    <main><router-outlet /></main>
  `,
})
export class App {
  protected api = inject(ApiService);
}
