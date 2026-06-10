import React from 'react';
import ReactDOM from 'react-dom/client';
import { Hud } from './app/Hud.js';
import './app/styles.css';

const root = document.getElementById('root');
if (!root) throw new Error('missing #root element');

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <Hud />
  </React.StrictMode>,
);
