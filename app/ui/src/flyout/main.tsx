import '../shared/base.css';
import '../shared/ui.css';
import './flyout.css';
import { applyRemembered } from '../shared/theme';
import { createRoot } from 'react-dom/client';
import { Flyout } from './Flyout';

applyRemembered();
createRoot(document.getElementById('root')!).render(<Flyout />);
