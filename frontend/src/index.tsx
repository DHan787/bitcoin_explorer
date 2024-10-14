/*
 * @Author: Jiang Han
 * @Date: 2024-09-29 21:23:04
 * @Description: 
 */
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';

// Use ReactDOM.createRoot instead of ReactDOM.render
const root = ReactDOM.createRoot(document.getElementById('root') as HTMLElement);
root.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
