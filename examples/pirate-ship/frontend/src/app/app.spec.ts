import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient, withInterceptors } from '@angular/common/http';
import { TestBed } from '@angular/core/testing';
import { Router, provideRouter } from '@angular/router';
import { ApiService, Me, authInterceptor } from './api.service';
import { App } from './app';
import { routes } from './app.routes';
import { CrewScreen } from './screens/crew';
import { PlunderScreen } from './screens/plunder';
import { VoyagesScreen } from './screens/voyages';

const voyage = {
  id: 1,
  name: 'Main',
  destination: 'Nassau',
  state: 'docked',
  next_states: ['sailing'],
  events: [],
  roster: [],
};
const person = (orders: string[]): Me => ({
  id: 1,
  name: 'Anne',
  rank: 'captain',
  status: 'aboard',
  shares_display: '2',
  orders,
});

describe('the deck', () => {
  let http: HttpTestingController;
  let api: ApiService;
  let router: Router;

  const settle = () => new Promise((resolve) => setTimeout(resolve));

  beforeEach(() => {
    sessionStorage.clear();
    localStorage.clear();
    TestBed.configureTestingModule({
      providers: [
        provideRouter(routes),
        provideHttpClient(withInterceptors([authInterceptor])),
        provideHttpClientTesting(),
      ],
    });
    http = TestBed.inject(HttpTestingController);
    api = TestBed.inject(ApiService);
    router = TestBed.inject(Router);
  });

  it('TEST-0059: without a session every route goes to the login', async () => {
    for (const url of ['/', '/crew', '/voyages', '/hold', '/voyages/1/plunder']) {
      await router.navigateByUrl(url);
      expect(router.url).toBe('/login');
    }
  });

  it('TEST-0060: a 401 from the API ends the session and goes to the login', async () => {
    api.token.set('old');
    api.me.set(person([]));
    sessionStorage.setItem('ledger-token', 'old');
    const call = api.crew();
    http.expectOne('/api/crew').flush({ error: 'not_logged_in', message: 'no' }, { status: 401, statusText: 'x' });
    await call;
    await settle();
    expect(api.token()).toBeNull();
    expect(sessionStorage.getItem('ledger-token')).toBeNull();
    expect(router.url).toBe('/login');
  });

  it('TEST-0061: a button for an order the rank may not give is not shown', async () => {
    for (const [orders, buttons] of [
      [['move_ship'], true],
      [[], false],
    ] as const) {
      api.me.set(person([...orders]));
      const fixture = TestBed.createComponent(VoyagesScreen);
      fixture.detectChanges();
      http.expectOne('/api/voyages').flush([voyage]);
      await settle();
      fixture.detectChanges();
      const text = fixture.nativeElement.textContent as string;
      expect(text.includes('Hoist the sails')).toBe(buttons);
      expect(text.includes('Chart a new voyage')).toBe(buttons);
      fixture.destroy();
    }
  });

  it('TEST-0062: money is shown with the display string of the API, never computed', async () => {
    api.me.set(person([]));
    const fixture = TestBed.createComponent(PlunderScreen);
    fixture.componentRef.setInput('id', '1');
    fixture.detectChanges();
    http.expectOne('/api/voyages/1').flush(voyage);
    await settle();
    http
      .expectOne('/api/voyages/1/plunder')
      .flush({ plunder: [{ description: 'galleon', amount_display: 'SHOWN AS GIVEN' }], compensations: [] });
    await settle();
    http.expectOne('/api/voyages/1/split').flush({ error: 'not_found', message: 'no' }, { status: 404, statusText: 'x' });
    await settle();
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent).toContain('SHOWN AS GIVEN');
  });

  it('TEST-0063: a refusal shows its message in a bottle, not its code', async () => {
    const fixture = TestBed.createComponent(App);
    const call = api.addMember('Bo', 'captain');
    http.expectOne('/api/crew').flush({ error: 'rank_taken', message: 'Too many officers!' }, { status: 409, statusText: 'x' });
    await call;
    fixture.detectChanges();
    const bottle = fixture.nativeElement.querySelector('[role=alert]');
    expect(bottle.textContent).toContain('Too many officers!');
    expect(bottle.textContent).not.toContain('rank_taken');
  });

  it('TEST-0065: a refused rank or status goes back to what the back says', async () => {
    api.me.set(person(['manage_crew']));
    const anne = { id: 1, name: 'Anne', rank: 'captain', status: 'aboard', shares_display: '2' };
    const fixture = TestBed.createComponent(CrewScreen);
    fixture.detectChanges();
    http.expectOne('/api/crew').flush([anne]);
    await settle();
    fixture.detectChanges();

    const select: HTMLSelectElement = fixture.nativeElement.querySelector('[aria-label="Status of Anne"]');
    select.value = 'dead';
    select.dispatchEvent(new Event('change'));
    http
      .expectOne('/api/crew/1')
      .flush({ error: 'captain_required', message: 'Make another captain first.' }, { status: 409, statusText: 'x' });
    await settle();
    http.expectOne('/api/crew').flush([anne]);
    await settle();
    expect(select.value).toBe('aboard');
    expect(api.bottle()).toBe('Make another captain first.');
  });

  it('TEST-0064: the token lives in memory and sessionStorage, and goes out as a Bearer', async () => {
    const login = api.login('anne', 'seven-seas-rum');
    http.expectOne('/api/login').flush({ token: 'abc', me: person([]) });
    await login;
    expect(api.token()).toBe('abc');
    expect(sessionStorage.getItem('ledger-token')).toBe('abc');
    expect(localStorage.length).toBe(0);
    expect(document.cookie).toBe('');

    void api.hold();
    const request = http.expectOne('/api/hold');
    expect(request.request.headers.get('Authorization')).toBe('Bearer abc');
    request.flush({ counts: {}, weight_stone: 0, capacity_stone: 2000 });
  });
});
