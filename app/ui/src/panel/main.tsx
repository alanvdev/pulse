import '../shared/base.css';
import '../shared/ui.css';
import './panel.css';
import { applyRemembered } from '../shared/theme';
import { createRoot } from 'react-dom/client';
import { SP_MOVE } from '../shared/motion';
import { Bolsillo } from './Bolsillo';

document.documentElement.style.setProperty('--sp', SP_MOVE.e);
applyRemembered();
createRoot(document.getElementById('root')!).render(<Bolsillo />);
