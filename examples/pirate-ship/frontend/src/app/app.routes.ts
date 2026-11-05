import { Routes } from '@angular/router';
import { authGuard } from './api.service';
import { ChartScreen } from './screens/chart';
import { CrewScreen } from './screens/crew';
import { HoldScreen } from './screens/hold';
import { LoginScreen } from './screens/login';
import { PlunderScreen } from './screens/plunder';
import { VoyagesScreen } from './screens/voyages';

export const routes: Routes = [
  { path: 'login', component: LoginScreen, title: 'Board the Ship' },
  { path: '', component: ChartScreen, canActivate: [authGuard], title: 'The Chart' },
  { path: 'crew', component: CrewScreen, canActivate: [authGuard], title: "Ship's Articles" },
  { path: 'voyages', component: VoyagesScreen, canActivate: [authGuard], title: 'The Course' },
  { path: 'voyages/:id/plunder', component: PlunderScreen, canActivate: [authGuard], title: 'Divide the Plunder' },
  { path: 'hold', component: HoldScreen, canActivate: [authGuard], title: 'Below Deck' },
  { path: '**', redirectTo: '' },
];
