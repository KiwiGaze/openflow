import React from 'react';
import ReactDOM from 'react-dom/client';
import { Scratchpad } from './app/Scratchpad.js';
import './app/styles.css';

const root = document.getElementById('root');
if (!root) throw new Error('missing #root element');

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <Scratchpad />
  </React.StrictMode>,
);
