import '../shared/base.css';
import '../shared/ui.css';
import './dashboard.css';
import { applyRemembered } from '../shared/theme';
import { createRoot } from 'react-dom/client';
import { Mosaico } from './Mosaico';

applyRemembered();
createRoot(document.getElementById('root')!).render(<Mosaico />);
