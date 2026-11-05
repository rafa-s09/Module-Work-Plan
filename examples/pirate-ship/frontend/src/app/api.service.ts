import { HttpClient, HttpErrorResponse, HttpInterceptorFn } from '@angular/common/http';
import { Injectable, computed, inject, signal } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { Observable, catchError, firstValueFrom, of, throwError } from 'rxjs';

export interface Me {
  id: number;
  name: string;
  rank: string;
  status: string;
  shares_display: string;
  orders: string[];
}
export type Member = Omit<Me, 'orders'>;
export interface RosterLine {
  crew_member_id: number;
  name: string;
  status: string;
  shares_display: string;
}
export interface Voyage {
  id: number;
  name: string;
  destination: string;
  state: string;
  next_states: string[];
  events: { state: string; at: string }[];
  roster: RosterLine[];
}
export interface Hold {
  counts: Record<string, number>;
  weight_stone: number;
  capacity_stone: number;
}
export interface Chest {
  balance_reales: number;
  balance_display: string;
}
export interface Plunder {
  plunder: { description: string; amount_display: string }[];
  compensations: { name: string; injury: string; amount_display: string }[];
}
export interface SplitLine {
  crew_member_id: number;
  name: string;
  shares_display: string;
  paid_to: string;
  compensation_display: string;
  share_display: string;
  total_display: string;
}
export interface Split {
  plunder_display: string;
  compensation_display: string;
  leftover_display: string;
  lines: SplitLine[];
}

const TOKEN_KEY = 'ledger-token';

/** The only door to the back. Every call answers null on a refusal, and the refusal goes in the bottle. */
@Injectable({ providedIn: 'root' })
export class ApiService {
  private http = inject(HttpClient);
  private router = inject(Router);

  readonly me = signal<Me | null>(null);
  readonly bottle = signal('');
  readonly token = signal(sessionStorage.getItem(TOKEN_KEY));
  readonly loggedIn = computed(() => this.token() !== null && this.me() !== null);

  can(order: string): boolean {
    return this.me()?.orders.includes(order) ?? false;
  }

  private call<T>(request: Observable<T>): Promise<T | null> {
    this.bottle.set('');
    return firstValueFrom(
      request.pipe(
        catchError((error: HttpErrorResponse) => {
          this.bottle.set(error.error?.message ?? 'The trumpet is silent. Is the back afloat?');
          return of(null);
        }),
      ),
    );
  }

  endSession(): void {
    sessionStorage.removeItem(TOKEN_KEY);
    this.token.set(null);
    this.me.set(null);
    void this.router.navigateByUrl('/login');
  }

  async login(username: string, password: string): Promise<boolean> {
    const answer = await this.call(
      this.http.post<{ token: string; me: Me }>('/api/login', { username, password }),
    );
    if (!answer) return false;
    sessionStorage.setItem(TOKEN_KEY, answer.token);
    this.token.set(answer.token);
    this.me.set(answer.me);
    return true;
  }

  async logout(): Promise<void> {
    await this.call(this.http.post('/api/logout', null));
    this.endSession();
  }

  async loadMe(): Promise<void> {
    this.me.set(await this.call(this.http.get<Me>('/api/me')));
  }

  changeOwnPassword(current_password: string, new_password: string) {
    return this.call(this.http.put('/api/me/password', { current_password, new_password }, { observe: 'response' }));
  }
  createAccount(crew_member_id: number, username: string, password: string) {
    return this.call(this.http.post('/api/accounts', { crew_member_id, username, password }));
  }
  resetPassword(memberId: number, new_password: string) {
    return this.call(this.http.put(`/api/accounts/${memberId}/password`, { new_password }, { observe: 'response' }));
  }

  crew() {
    return this.call(this.http.get<Member[]>('/api/crew'));
  }
  addMember(name: string, rank: string) {
    return this.call(this.http.post<Member>('/api/crew', { name, rank }));
  }
  changeMember(id: number, change: { rank?: string; status?: string }) {
    return this.call(this.http.patch<Member>(`/api/crew/${id}`, change));
  }

  voyages() {
    return this.call(this.http.get<Voyage[]>('/api/voyages'));
  }
  voyage(id: number) {
    return this.call(this.http.get<Voyage>(`/api/voyages/${id}`));
  }
  createVoyage(name: string, destination: string) {
    return this.call(this.http.post<Voyage>('/api/voyages', { name, destination }));
  }
  changeVoyageState(id: number, state: string) {
    return this.call(this.http.post<Voyage>(`/api/voyages/${id}/state`, { state }));
  }

  hold() {
    return this.call(this.http.get<Hold>('/api/hold'));
  }
  changeHold(item: string, amount: number, reason: string) {
    return this.call(this.http.post<Hold>('/api/hold/changes', { item, amount, reason }));
  }

  chest() {
    return this.call(this.http.get<Chest>('/api/chest'));
  }
  plunder(voyageId: number) {
    return this.call(this.http.get<Plunder>(`/api/voyages/${voyageId}/plunder`));
  }
  addPlunder(voyageId: number, amount_reales: number, description: string) {
    return this.call(this.http.post<Plunder>(`/api/voyages/${voyageId}/plunder`, { amount_reales, description }));
  }
  addCompensation(voyageId: number, crew_member_id: number, injury: string) {
    return this.call(this.http.post<Plunder>(`/api/voyages/${voyageId}/compensations`, { crew_member_id, injury }));
  }
  runSplit(voyageId: number) {
    return this.call(this.http.post<Split>(`/api/voyages/${voyageId}/split`, null));
  }
  /** A voyage not yet split answers 404: that is no refusal, so it stays out of the bottle. */
  split(voyageId: number): Promise<Split | null> {
    return firstValueFrom(
      this.http.get<Split>(`/api/voyages/${voyageId}/split`).pipe(catchError(() => of(null))),
    );
  }
}

/** Adds the token to every call; a 401 on any call but login ends the session (deck#FR-02). */
export const authInterceptor: HttpInterceptorFn = (request, next) => {
  const api = inject(ApiService);
  const token = api.token();
  const signed = token ? request.clone({ setHeaders: { Authorization: `Bearer ${token}` } }) : request;
  return next(signed).pipe(
    catchError((error: HttpErrorResponse) => {
      if (error.status === 401 && !request.url.endsWith('/login')) api.endSession();
      return throwError(() => error);
    }),
  );
};

export const authGuard: CanActivateFn = async () => {
  const api = inject(ApiService);
  const router = inject(Router);
  if (!api.token()) return router.parseUrl('/login');
  if (!api.me()) await api.loadMe();
  return api.me() ? true : router.parseUrl('/login');
};
