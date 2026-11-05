import { Component, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { ApiService } from '../api.service';

@Component({
  selector: 'app-login',
  imports: [FormsModule],
  template: `
    <section class="card boarding">
      <img src="theme/compass-rose.svg" alt="" width="96" height="96" class="turning" />
      <h1>Board the Ship</h1>
      <p>State yer name and the password, or walk back to shore.</p>
      <form (ngSubmit)="board()">
        <label>Username <input name="username" [(ngModel)]="username" autocomplete="username" required /></label>
        <label>
          Password
          <input name="password" type="password" [(ngModel)]="password" autocomplete="current-password" required />
        </label>
        <button type="submit">Come aboard</button>
      </form>
    </section>
  `,
})
export class LoginScreen {
  private api = inject(ApiService);
  private router = inject(Router);
  username = '';
  password = '';

  async board() {
    if (await this.api.login(this.username, this.password)) await this.router.navigateByUrl('/');
    this.password = '';
  }
}
