import '../shared/base.css';
import '../shared/ui.css';
import './flyout.css';
import { createRoot } from 'react-dom/client';
import { Flyout } from './Flyout';

createRoot(document.getElementById('root')!).render(<Flyout />);
