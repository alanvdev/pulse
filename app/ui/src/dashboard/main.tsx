import '../shared/base.css';
import '../shared/ui.css';
import './dashboard.css';
import { createRoot } from 'react-dom/client';
import { Mosaico } from './Mosaico';

createRoot(document.getElementById('root')!).render(<Mosaico />);
