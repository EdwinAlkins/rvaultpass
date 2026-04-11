import { render } from 'preact';
import { I18nextProvider } from 'react-i18next';
import i18n from './i18n';
import App from './App';
import './App.css';

render(
  <I18nextProvider i18n={i18n}>
    <App />
  </I18nextProvider>,
  document.getElementById('root')!
);
